use crate::utils::{
    is_brace_macro_expr, is_stray_semicolon, line_column, parse_token_tree, trailing_jump_stmt,
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
    /// A `use` item in its canonical form, see `canonical_use`
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
        return Err(format!(
            "the syntax tree differs: {a:?} at input {} vs. {b:?} at output {}",
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

/// Whether `node` is the block of a closure body or of a match arm body that holds just an
/// expression (or a jump statement), `|x| { x }`, `A => { x }`, or such a block in another one, `A => { { x } }`: the
/// formatter removes its braces or adds them, so it is compared as if it had none
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
    // No label or modifier in front of `{`, and only an expression in it, not a statement `m! {}`;
    // or only `return x;`, compared as the expression `return x` (see `is_trailing_jump_stmt`)
    node.first_token() == stmt_list.first_token()
        && stmt_list.children().count() == 1
        && stmt_list.children().all(|c| {
            (ast::Expr::can_cast(c.kind()) && !is_brace_macro_expr(&c)) || is_trailing_jump_stmt(&c)
        })
}

/// The preorder walk of the tree without trivia and without trailing commas, with source offsets
fn steps(file: &SourceFile, edition: Edition) -> Vec<(Step, TextSize)> {
    let mut out = Vec::new();
    push_steps(file.syntax(), edition, &|offset| offset, &mut out);
    sort_use_runs(&mut out);
    out
}

/// The steps of the walk of `root` (see `steps`), with offsets mapped to the source by `source_offset`
fn push_steps(
    root: &SyntaxNode,
    edition: Edition,
    source_offset: &dyn Fn(TextSize) -> TextSize,
    out: &mut Vec<(Step, TextSize)>,
) {
    // The blocks of `is_optional_block` and their statement lists, walked as if not there
    let mut transparent: Vec<SyntaxNode> = Vec::new();
    let mut walk = root.preorder_with_tokens();
    while let Some(event) = walk.next() {
        match event {
            // The arguments of a macro call that parse as expressions are formatted as such (see
            // `format_macro_call`), so they are compared parsed
            WalkEvent::Enter(NodeOrToken::Node(node)) if node.kind() == SyntaxKind::TOKEN_TREE => {
                if let Some(parsed) = parse_macro_arguments(&node, edition) {
                    let (start, parsed_start) =
                        (node.text_range().start(), parsed.text_range().start());
                    let offset = |o: TextSize| source_offset(start + (o - parsed_start));
                    push_steps(&parsed, edition, &offset, out);
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
            WalkEvent::Enter(NodeOrToken::Node(node))
            | WalkEvent::Leave(NodeOrToken::Node(node))
                if transparent.contains(&node) => {}
            WalkEvent::Enter(NodeOrToken::Token(token))
                if token.parent().is_some_and(|p| transparent.contains(&p)) => {}
            WalkEvent::Enter(NodeOrToken::Node(node)) if node.kind() == SyntaxKind::USE => {
                // Imports are sorted and normalized, so they are compared in a canonical form
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
                // A shebang includes its line break, which is layout
                let text = token.text().trim_end().to_string();
                out.push((
                    Step::Token(token.kind(), text),
                    source_offset(token.text_range().start()),
                ))
            }
            _ => {}
        }
    }
}

/// The token tree of a macro call parsed as the formatter parses it (see `format_token_tree`): the
/// arguments of a call for `(...)`, an array for `[...]`
fn parse_macro_arguments(node: &SyntaxNode, edition: Edition) -> Option<SyntaxNode> {
    if node.parent()?.kind() != SyntaxKind::MACRO_CALL {
        return None;
    }
    let tree = ast::TokenTree::cast(node.clone())?;
    match node.first_token()?.kind() {
        SyntaxKind::L_PAREN => {
            parse_token_tree::<ast::ArgList>(&tree, edition).map(|n| n.syntax().clone())
        }
        SyntaxKind::L_BRACK => {
            parse_token_tree::<ast::ArrayExpr>(&tree, edition).map(|n| n.syntax().clone())
        }
        _ => None,
    }
}

/// Consecutive `use` items may be reordered, and an empty one (`use a::{};`) removed, so each run of
/// them is compared as a sorted list without the empty ones
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

/// The source text of `node`, only the compared tokens (`is_compared_token`): `#[cfg(any(a, b,))]` is
/// `#[cfg(any(a,b))]`
fn code_text(node: &SyntaxNode) -> String {
    node.descendants_with_tokens()
        .filter_map(NodeOrToken::into_token)
        .filter(is_compared_token)
        .map(|t| t.text().to_string())
        .collect()
}

/// `#[attr] pub use tree` with the tree in its canonical form (see `canonical_use_tree`); empty if the
/// item imports nothing and has no attributes (an item with attributes is never removed)
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

/// The tree in a normal form: lists sorted, empty trees removed and `a::{b}` written as `a::b` (except
/// `a::{self}`); `None` if the tree imports nothing
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

/// The statement `return;`, `break;` or `continue;` ending a block (see `trailing_jump_stmt`)
fn is_trailing_jump_stmt(node: &SyntaxNode) -> bool {
    node.parent()
        .and_then(ast::StmtList::cast)
        .and_then(|list| trailing_jump_stmt(&list))
        .is_some_and(|stmt| stmt.syntax() == node)
}

/// A token that is compared: not trivia, and not a token the formatter adds or removes (a trailing
/// comma, the leading `|` of an or-pattern, a stray `;`)
fn is_compared_token(token: &SyntaxToken) -> bool {
    !token.kind().is_trivia()
        && !is_trailing_comma(token)
        && !is_leading_pipe(token)
        && !is_stray_semicolon(token)
}

/// The `|` in front of the first pattern of an or-pattern, `| A | B`, which the formatter drops
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
            .find(|e| !e.kind().is_trivia())
            // The closing delimiter of the list (`)`, `]`, `}`, `>`, the `|` of closure parameters), or
            // nothing: a comma ending its node, the last item of a `where` clause
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
        .filter(|t| t.kind() == SyntaxKind::COMMENT)
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
}
