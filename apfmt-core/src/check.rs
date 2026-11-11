use crate::comments::{is_code_kind, is_comment_kind};
use crate::syntax_utils::{
    MacroArgs, MacroRule, Placeholders, Reparsed, is_brace_macro, is_empty_where_clause,
    is_stray_semicolon, line_column, macro_rules_rules, parse_macro_args, parse_transcriber,
    trailing_jump_stmt,
};
use ra_ap_syntax::ast::{self, HasAttrs, HasVisibility, Use, UseTree};
use ra_ap_syntax::{
    AstNode, Direction, Edition, NodeOrToken, SourceFile, SyntaxKind, SyntaxNode, SyntaxToken,
    TextSize, WalkEvent,
};

#[derive(PartialEq, Eq, Debug)]
enum Step {
    Enter(SyntaxKind),
    Leave(SyntaxKind),
    Token(SyntaxKind, String),
    Use(String),
}

pub(crate) fn check_result(
    input: &SourceFile,
    output: &SourceFile,
    input_text: &str,
    output_text: &str,
    edition: Edition,
) -> Result<(), String> {
    let input_steps = steps(input, edition);
    let output_steps = steps(output, edition);
    let mismatch = input_steps
        .iter()
        .zip(&output_steps)
        .find(|((a, _), (b, _))| a != b);
    if let Some(((a, a_offset), (b, b_offset))) = mismatch {
        // A macro argument is compared parsed only if formatting keeps its punctuation spacing
        let tree = Step::Enter(SyntaxKind::TOKEN_TREE);
        let hint = if (*a == tree) != (*b == tree) {
            " (macro tokens parsed on one side only: their punctuation spacing has changed)"
        } else {
            ""
        };
        return Err(format!(
            "the syntax tree differs: {a:?} at input {} vs. {b:?} at output {}{hint}",
            position(input_text, *a_offset),
            position(output_text, *b_offset)
        ));
    }

    let input_comments = comments(input);
    let output_comments = comments(output);
    if input_comments != output_comments {
        let missing: Vec<_> = input_comments
            .iter()
            .filter(|c| !output_comments.contains(c))
            .collect();
        let extra: Vec<_> = output_comments
            .iter()
            .filter(|c| !input_comments.contains(c))
            .collect();
        return Err(format!(
            "the comments differ (compared without whitespace): missing {missing:?}, added {extra:?}"
        ));
    }
    Ok(())
}

/// A block of a closure or match arm body holding just an expression (`|x| { x }`, also nested):
/// the formatter removes or adds its braces, so it is compared as if it had none
fn is_optional_block(node: &SyntaxNode) -> bool {
    if node.kind() != SyntaxKind::BLOCK_EXPR {
        return false;
    }
    let optional_position = node.parent().is_some_and(|p| match p.kind() {
        SyntaxKind::CLOSURE_EXPR | SyntaxKind::MATCH_ARM => true,
        SyntaxKind::STMT_LIST => p.parent().is_some_and(|b| is_optional_block(&b)),
        _ => false,
    });
    if !optional_position {
        return false;
    }
    let Some(stmt_list) = node
        .first_child()
        .filter(|c| c.kind() == SyntaxKind::STMT_LIST)
    else {
        return false;
    };
    // A plain block with only an expression, not `m! {}`, or only `return x;`
    node.first_token() == stmt_list.first_token()
        && stmt_list.children().count() == 1
        && stmt_list.children().all(|c| {
            ast::Expr::cast(c.clone()).is_some_and(|e| !is_brace_macro(&e))
                || is_trailing_jump_stmt(&c)
        })
}

/// Preorder, without trivia and trailing commas, with source offsets
fn steps(file: &SourceFile, edition: Edition) -> Vec<(Step, TextSize)> {
    let mut out = Vec::new();
    let placeholders = Placeholders::default();
    push_steps(
        file.syntax(),
        edition,
        &|offset| offset,
        &placeholders,
        &mut out,
    );
    sort_use_runs(&mut out);
    out
}

/// Offsets mapped by `source_offset`; placeholders are compared as their source text
fn push_steps(
    root: &SyntaxNode,
    edition: Edition,
    source_offset: &dyn Fn(TextSize) -> TextSize,
    placeholders: &Placeholders,
    out: &mut Vec<(Step, TextSize)>,
) {
    let mut transparent: Vec<SyntaxNode> = Vec::new();
    let mut walk = root.preorder_with_tokens();
    while let Some(event) = walk.next() {
        match event {
            // Macro arguments that parse as expressions are compared parsed; `macro_rules!` rule by
            // rule
            WalkEvent::Enter(NodeOrToken::Node(node)) if node.kind() == SyntaxKind::TOKEN_TREE => {
                let tree = ast::TokenTree::cast(node.clone()).unwrap();
                if let Some(parsed) = parse_macro_arguments(&tree, edition, placeholders) {
                    push_reparsed_steps(&node, parsed, edition, source_offset, out);
                    walk.skip_subtree();
                } else if let Some(rules) = macro_rules_tree_rules(&tree) {
                    let rules = MacroRulesSteps {
                        tree: &tree,
                        rules: &rules,
                        placeholders,
                    };
                    rules.push(edition, source_offset, out);
                    walk.skip_subtree();
                } else {
                    out.push((
                        Step::Enter(node.kind()),
                        source_offset(node.text_range().start()),
                    ))
                }
            }
            WalkEvent::Enter(NodeOrToken::Node(node)) if is_optional_block(&node) => {
                transparent.push(node.first_child().unwrap());
                transparent.push(node);
            }
            // `break;` is compared as the tail expression `break`, its `;` dropped
            WalkEvent::Enter(NodeOrToken::Node(node)) if is_trailing_jump_stmt(&node) => {
                transparent.push(node);
            }
            WalkEvent::Enter(NodeOrToken::Node(node)) if is_empty_where_clause(&node) => {
                transparent.push(node);
            }
            WalkEvent::Enter(NodeOrToken::Node(node))
            | WalkEvent::Leave(NodeOrToken::Node(node))
                if transparent.contains(&node) => {}
            WalkEvent::Enter(NodeOrToken::Token(token))
                if token.parent().is_some_and(|p| transparent.contains(&p)) => {}
            WalkEvent::Enter(NodeOrToken::Node(node)) if node.kind() == SyntaxKind::USE => {
                let canonical = canonical_use(&Use::cast(node.clone()).unwrap());
                out.push((
                    Step::Use(canonical),
                    source_offset(node.text_range().start()),
                ));
                walk.skip_subtree();
            }
            WalkEvent::Leave(NodeOrToken::Node(node)) if node.kind() == SyntaxKind::USE => {}
            WalkEvent::Enter(NodeOrToken::Node(node)) => out.push((
                Step::Enter(node.kind()),
                source_offset(node.text_range().start()),
            )),
            WalkEvent::Leave(NodeOrToken::Node(node)) => {
                // `extern` is `extern "C"`: the formatter writes the ABI out
                if node.kind() == SyntaxKind::ABI
                    && !node
                        .children_with_tokens()
                        .any(|c| c.kind() == SyntaxKind::STRING)
                {
                    let step = Step::Token(SyntaxKind::STRING, "\"C\"".to_string());
                    out.push((step, source_offset(node.text_range().end())));
                }
                out.push((
                    Step::Leave(node.kind()),
                    source_offset(node.text_range().end()),
                ))
            }
            WalkEvent::Enter(NodeOrToken::Token(token)) if is_compared_token(&token) => {
                out.push(token_step(&token, source_offset, placeholders))
            }
            _ => {}
        }
    }
}

/// A placeholder is compared as the text it stands for
fn token_step(
    token: &SyntaxToken,
    source_offset: &dyn Fn(TextSize) -> TextSize,
    placeholders: &Placeholders,
) -> (Step, TextSize) {
    // A shebang includes its line break, which is layout
    let text = match placeholders.get(token) {
        Some(source) => source.clone(),
        None => token.text().trim_end().to_string(),
    };
    let step = Step::Token(token.kind(), text);
    (step, source_offset(token.text_range().start()))
}

fn push_reparsed_steps(
    node: &SyntaxNode,
    parsed: Reparsed<SyntaxNode>,
    edition: Edition,
    source_offset: &dyn Fn(TextSize) -> TextSize,
    out: &mut Vec<(Step, TextSize)>,
) {
    let (start, parsed_start) = (node.text_range().start(), parsed.node.text_range().start());
    let offset = |o: TextSize| source_offset(start + (o - parsed_start));
    let placeholders = parsed.placeholders.into_iter().collect();
    push_steps(&parsed.node, edition, &offset, &placeholders, out);
}

/// `(...)` as call arguments, `[...]` as an array, as the formatter parses it
fn parse_macro_arguments(
    tree: &ast::TokenTree,
    edition: Edition,
    placeholders: &Placeholders,
) -> Option<Reparsed<SyntaxNode>> {
    if tree.syntax().parent()?.kind() != SyntaxKind::MACRO_CALL {
        return None;
    }
    parse_macro_args(tree, edition, placeholders).map(|parsed| parsed.map(MacroArgs::into_syntax))
}

fn macro_rules_tree_rules(tree: &ast::TokenTree) -> Option<Vec<MacroRule>> {
    if tree.syntax().parent()?.kind() != SyntaxKind::MACRO_RULES {
        return None;
    }
    macro_rules_rules(tree)
}

/// Rule by rule, without the `;` the formatter writes after each; a transcriber that parses is
/// compared parsed
struct MacroRulesSteps<'a> {
    tree: &'a ast::TokenTree,
    rules: &'a [MacroRule],
    placeholders: &'a Placeholders,
}

impl MacroRulesSteps<'_> {
    fn push(
        &self,
        edition: Edition,
        source_offset: &dyn Fn(TextSize) -> TextSize,
        out: &mut Vec<(Step, TextSize)>,
    ) {
        let node = self.tree.syntax();
        let placeholders = self.placeholders;
        out.push((
            Step::Enter(node.kind()),
            source_offset(node.text_range().start()),
        ));
        out.push(token_step(
            &node.first_token().unwrap(),
            source_offset,
            placeholders,
        ));
        for rule in self.rules {
            push_steps(
                rule.matcher.syntax(),
                edition,
                source_offset,
                placeholders,
                out,
            );
            out.push(token_step(&rule.arrow.0, source_offset, placeholders));
            out.push(token_step(&rule.arrow.1, source_offset, placeholders));
            let transcriber = rule.transcriber.syntax();
            match parse_transcriber(rule, edition, placeholders) {
                Some(parsed) => {
                    push_reparsed_steps(transcriber, parsed, edition, source_offset, out)
                }
                None => push_steps(transcriber, edition, source_offset, placeholders, out),
            }
        }
        out.push(token_step(
            &node.last_token().unwrap(),
            source_offset,
            placeholders,
        ));
        out.push((
            Step::Leave(node.kind()),
            source_offset(node.text_range().end()),
        ));
    }
}

/// Each run of `use` items is compared sorted without empty ones, since they may be reordered or
/// removed
fn sort_use_runs(steps: &mut Vec<(Step, TextSize)>) {
    steps.retain(|(step, _)| !matches!(step, Step::Use(text) if text.is_empty()));
    let is_use = |step: &(Step, TextSize)| matches!(step.0, Step::Use(_));
    let mut start = 0;
    while start < steps.len() {
        let len = steps[start..].iter().take_while(|s| is_use(s)).count();
        steps[start..start + len].sort_by(|(a, _), (b, _)| match (a, b) {
            (Step::Use(a), Step::Use(b)) => a.cmp(b),
            _ => unreachable!(),
        });
        start += len.max(1);
    }
}

/// Only the compared tokens: `#[cfg(any(a, b,))]` is `#[cfg(any(a,b))]`
fn code_text(node: &SyntaxNode) -> String {
    node.descendants_with_tokens()
        .filter_map(NodeOrToken::into_token)
        .filter(is_compared_token)
        .map(|t| t.text().to_string())
        .collect()
}

/// With the tree in canonical form; empty if it imports nothing and has no attributes
fn canonical_use(item: &Use) -> String {
    let attrs: Vec<String> = item.attrs().map(|a| code_text(a.syntax())).collect();
    let tree = item.use_tree().as_ref().and_then(canonical_use_tree);
    let Some(tree) = tree.or_else(|| (!attrs.is_empty()).then(|| "{}".to_string())) else {
        return String::new();
    };
    let visibility = item.visibility().map(|v| code_text(v.syntax()));
    format!(
        "{} {} use {tree}",
        attrs.join(" "),
        visibility.unwrap_or_default()
    )
}

/// Lists sorted, empty trees removed, `a::{b}` as `a::b` (except `a::{self}`); `None` if it imports
/// nothing
fn canonical_use_tree(tree: &UseTree) -> Option<String> {
    let mut text = tree
        .path()
        .map(|p| code_text(p.syntax()))
        .unwrap_or_default();
    if tree.coloncolon_token().is_some() {
        text.push_str("::");
    }
    if tree.star_token().is_some() {
        text.push('*');
    }
    if let Some(list) = tree.use_tree_list() {
        let mut items: Vec<String> = list
            .use_trees()
            .filter_map(|t| canonical_use_tree(&t))
            .collect();
        items.sort();
        match items.as_slice() {
            [] => return None,
            [item] if item != "self" && !item.starts_with("self as ") => text.push_str(item),
            _ => text.push_str(&format!("{{{}}}", items.join(","))),
        }
    }
    if let Some(rename) = tree.rename() {
        let name = code_text(rename.syntax());
        text.push_str(&format!(" as {}", name.strip_prefix("as").unwrap_or(&name)));
    }
    Some(text)
}

fn is_trailing_jump_stmt(node: &SyntaxNode) -> bool {
    node.parent()
        .and_then(ast::StmtList::cast)
        .and_then(|list| trailing_jump_stmt(&list))
        .is_some_and(|stmt| stmt.syntax() == node)
}

/// Not trivia, nor a token the formatter adds or removes (a trailing comma, a leading `|`, a stray
/// `;`)
fn is_compared_token(token: &SyntaxToken) -> bool {
    is_code_kind(token.kind())
        && !is_trailing_comma(token)
        && !is_leading_pipe(token)
        && !is_stray_semicolon(token)
}

fn is_leading_pipe(token: &SyntaxToken) -> bool {
    token.kind() == SyntaxKind::PIPE
        && token.parent().is_some_and(|p| {
            p.kind() == SyntaxKind::OR_PAT && p.first_token().as_ref() == Some(token)
        })
}

fn is_trailing_comma(token: &SyntaxToken) -> bool {
    token.kind() == SyntaxKind::COMMA
        && token
            .siblings_with_tokens(Direction::Next)
            .skip(1)
            .find(|e| is_code_kind(e.kind()))
            // The closing delimiter of the list, or nothing (the last item of a `where` clause)
            .is_none_or(|e| {
                e.as_token().is_some_and(|t| {
                    token.parent().and_then(|p| p.last_token()).as_ref() == Some(t)
                })
            })
}

fn comments(file: &SourceFile) -> Vec<String> {
    let mut out: Vec<String> = file
        .syntax()
        .descendants_with_tokens()
        .filter_map(NodeOrToken::into_token)
        .filter(|t| is_comment_kind(t.kind()))
        .map(|t| t.text().split_whitespace().collect())
        .collect();
    out.sort();
    out
}

fn position(code: &str, offset: TextSize) -> String {
    let (line, column) = line_column(code, offset);
    format!("{line}:{column}")
}

#[cfg(test)]
mod tests {
    use super::check_result;
    use ra_ap_syntax::{Edition, SourceFile};

    fn check_result_test(a: &str, b: &str) -> Result<(), String> {
        let file_a = SourceFile::parse(a, Edition::CURRENT);
        let file_b = SourceFile::parse(b, Edition::CURRENT);
        assert!(file_a.errors().is_empty());
        assert!(file_b.errors().is_empty());
        check_result(&file_a.tree(), &file_b.tree(), a, b, Edition::CURRENT)
    }

    #[test]
    fn layout_changes_pass() {
        let input = "fn f( a : u32 , b:u32, ) { g( 1,2 ) ; }";
        let output = "fn f(a: u32, b: u32) {\n    g(\n        1,\n        2,\n    );\n}";
        assert_eq!(check_result_test(input, output), Ok(()));
    }

    #[test]
    fn comments_may_move_and_be_normalized() {
        let input = "fn f() { g(1 /*a*/, 2); }";
        let output = "fn f() {\n    g(1, /* a */ 2);\n}";
        assert_eq!(check_result_test(input, output), Ok(()));
    }

    #[test]
    fn removed_nested_arm_blocks_pass() {
        let input = "fn f() { match x { A => { { { 1 } } } } }";
        let output = "fn f() {\n    match x {\n        A => 1,\n    }\n}";
        assert!(check_result_test(input, output).is_ok());
    }

    #[test]
    fn removed_block_with_statement_fails() {
        let input = "fn f() { match x { A => { { g(); 1 } } } }";
        let output = "fn f() {\n    match x {\n        A => 1,\n    }\n}";
        assert!(check_result_test(input, output).is_err());
    }

    #[test]
    fn removed_empty_stmts_pass() {
        let input = "fn f() { ;; g();; /* a */ ; // b\n}";
        let output = "fn f() {\n    g(); /* a */ // b\n}";
        assert!(check_result_test(input, output).is_ok());
    }

    #[test]
    fn removed_empty_where_passes() {
        let input = "fn f() where {}\nstruct S where;";
        let output = "fn f() {}\nstruct S;";
        assert!(check_result_test(input, output).is_ok());
    }

    #[test]
    fn removed_where_with_predicate_fails() {
        let error = check_result_test("fn f() where T: C {}", "fn f() {}").unwrap_err();
        assert!(error.contains("differs"), "{error}");
    }

    #[test]
    fn removed_where_before_comment_fails() {
        let result = check_result_test("fn f() where /* c */ {}", "fn f() /* c */ {}");
        assert!(result.is_err());
    }

    #[test]
    fn dropped_stmt_semicolon_fails() {
        let error = check_result_test("fn f() { g(); }", "fn f() { g() }").unwrap_err();
        assert!(error.contains("differs"), "{error}");
    }

    #[test]
    fn trailing_jump_semicolon_may_change() {
        let input = "fn f() { loop { g(); break } }";
        let output = "fn f() {\n    loop {\n        g();\n        break;\n    }\n}";
        assert_eq!(check_result_test(input, output), Ok(()));
        assert_eq!(check_result_test(output, input), Ok(()));
    }

    #[test]
    fn removed_braces_of_trailing_jump_pass() {
        let input = "fn f() { match x { A => { return g() } } }";
        let output =
            "fn f() {\n    match x {\n        A => {\n            return g();\n        }\n    }\n}";
        assert_eq!(check_result_test(input, output), Ok(()));
        let output = "fn f() {\n    match x {\n        A => return g(),\n    }\n}";
        let input = "fn f() { match x { A => { return g(); } } }";
        assert_eq!(check_result_test(input, output), Ok(()));
        let input = "fn f() { match x { A => { g(); } } }";
        assert!(check_result_test(input, "fn f() { match x { A => g() } }").is_err());
    }

    #[test]
    fn semicolon_after_other_tail_fails() {
        assert!(check_result_test("fn f() { g() }", "fn f() { g(); }").is_err());
        assert!(check_result_test("fn f() { break; g() }", "fn f() { break g() }").is_err());
    }

    #[test]
    fn removed_braces_of_brace_macro_fail() {
        let input = "fn f() { match x { A => { m! {} } } }";
        let output = "fn f() {\n    match x {\n        A => m! {},\n    }\n}";
        assert!(check_result_test(input, output).is_err());
        let input = "fn f() { let c = || { m! {} }; }";
        assert!(check_result_test(input, "fn f() {\n    let c = || m! {};\n}").is_err());
    }

    #[test]
    fn removed_braces_of_paren_macro_pass() {
        let input = "fn f() { match x { A => { m!() } } }";
        let output = "fn f() {\n    match x {\n        A => m!(),\n    }\n}";
        assert_eq!(check_result_test(input, output), Ok(()));
    }

    #[test]
    fn normalized_macro_arguments_pass() {
        let input = "fn f() { assert!(x.all(|a| { a.b() }), \"m\"); }";
        let output = "fn f() {\n    assert!(x.all(|a| a.b()), \"m\");\n}";
        assert_eq!(check_result_test(input, output), Ok(()));
        let input = "fn f() { m!(match x { A => { 1 } B => 2 }); }";
        let output = "fn f() {\n    m!(match x {\n        A => 1,\n        B => 2,\n    });\n}";
        assert_eq!(check_result_test(input, output), Ok(()));
        let input = "fn f() { v![g(1, 2), n!(h(3, 4))]; }";
        let output = "fn f() {\n    v![g(1, 2,), n!(h(3, 4,))];\n}";
        assert_eq!(check_result_test(input, output), Ok(()));
    }

    #[test]
    fn changed_macro_arguments_fail() {
        let error = check_result_test("fn f() { m!(a, |x| { x }); }", "fn f() { m!(a, |x| y); }")
            .unwrap_err();
        assert!(
            error.contains("at input 1:22 vs. Token(IDENT, \"y\") at output 1:20"),
            "{error}"
        );
        assert!(check_result_test("fn f() { m!(a, b); }", "fn f() { m!(a b); }").is_err());
        let input = "fn f() { m! { |x| { x } } }";
        assert!(check_result_test(input, "fn f() { m! { |x| x } }").is_err());
    }

    #[test]
    fn dropped_code_fails() {
        let error = check_result_test("pub fn f() {}", "fn f() {}").unwrap_err();
        assert!(error.contains("VISIBILITY"), "{error}");
    }

    #[test]
    fn dropped_required_comma_fails() {
        let error =
            check_result_test("fn f() { let (a,) = b; }", "fn f() { let (a) = b; }").unwrap_err();
        assert!(error.contains("TUPLE_PAT"), "{error}");
    }

    #[test]
    fn dropped_comment_fails() {
        let error =
            check_result_test("fn f() { g(); // c\n}", "fn f() {\n    g();\n}").unwrap_err();
        assert!(error.contains("missing [\"//c\"]"), "{error}");
    }

    #[test]
    fn truncated_output_fails() {
        let error = check_result_test("fn f() {}\nfn g() {}", "fn f() {}").unwrap_err();
        assert!(error.contains("Enter(FN) at input 2:1"), "{error}");
    }

    #[test]
    fn where_clause_trailing_comma_passes() {
        let input = "fn f<T>() where T: Clone {}";
        let output = "fn f<T>()\nwhere\n    T: Clone,\n{}";
        assert_eq!(check_result_test(input, output), Ok(()));
    }

    #[test]
    fn explicit_c_abi_passes() {
        assert_eq!(
            check_result_test("extern fn f() {}", "extern \"C\" fn f() {}"),
            Ok(())
        );
    }

    #[test]
    fn sorted_imports_pass() {
        let input = "use c;\nuse a::{d, b::{}, c};\nuse a::{b};\nuse x::{};";
        let output = "use a::b;\nuse a::{c, d};\nuse c;";
        assert_eq!(check_result_test(input, output), Ok(()));
    }

    #[test]
    fn import_attribute_trailing_comma_passes() {
        let input = "#[cfg(any(a, b))]\nuse c;";
        let output = "#[cfg(any(\n    a,\n    b,\n))]\nuse c;";
        assert_eq!(check_result_test(input, output), Ok(()));
    }

    #[test]
    fn changed_import_attribute_fails() {
        let error =
            check_result_test("#[cfg(any(a, b))]\nuse c;", "#[cfg(any(a))]\nuse c;").unwrap_err();
        assert!(error.contains("differs"), "{error}");
    }

    #[test]
    fn changed_import_fails() {
        let error = check_result_test("use a::{b, c};", "use a::{b, d};").unwrap_err();
        assert!(error.contains("Use"), "{error}");
        let error = check_result_test("use a::{self};", "use a;").unwrap_err();
        assert!(error.contains("Use"), "{error}");
    }

    #[test]
    fn dropped_import_attribute_fails() {
        let error = check_result_test("#[cfg(test)] use a;", "use a;").unwrap_err();
        assert!(error.contains("cfg(test)"), "{error}");
        let error = check_result_test("#[cfg(test)] use a::{};", "").unwrap_err();
        assert!(error.contains("cfg(test)"), "{error}");
    }

    #[test]
    fn imports_moved_across_items_fail() {
        let error = check_result_test("use a;\nfn f() {}\nuse b;", "use a;\nuse b;\nfn f() {}")
            .unwrap_err();
        assert!(error.contains("Use"), "{error}");
    }

    #[test]
    fn changed_abi_fails() {
        let error =
            check_result_test("extern fn f() {}", "extern \"system\" fn f() {}").unwrap_err();
        assert!(error.contains("STRING"), "{error}");
    }

    #[test]
    fn macro_punct_spacing_kept_passes() {
        let input = "fn f() { m!(a,&*x, b..=c, x?.y); }";
        let output = "fn f() {\n    m!(\n        a,\n        &*x,\n        b..=c,\n        x?\n            .y\n    );\n}";
        assert_eq!(check_result_test(input, output), Ok(()));
    }

    #[test]
    fn macro_punct_spacing_changed_fails() {
        for (input, output) in [
            ("fn f() { m!(&*x); }", "fn f() { m!(& *x); }"),
            ("fn f() { m!(a - -b); }", "fn f() { m!(a --b); }"),
        ] {
            let error = check_result_test(input, output).unwrap_err();
            assert!(error.contains("punctuation spacing"), "{error}");
        }
    }
}
