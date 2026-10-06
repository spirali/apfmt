use ra_ap_syntax::{
    AstNode, Direction, Edition, NodeOrToken, SourceFile, SyntaxKind, SyntaxToken, TextSize,
    WalkEvent,
};

#[derive(PartialEq, Eq, Debug)]
enum Step {
    Enter(SyntaxKind),
    Leave(SyntaxKind),
    Token(SyntaxKind, String),
}

pub(crate) fn check_result(
    input: &SourceFile,
    output: &SourceFile,
    input_text: &str,
    output_text: &str,
) -> Result<(), String> {
    let input_steps = steps(&input);
    let output_steps = steps(&output);
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

    let input_comments = comments(&input);
    let output_comments = comments(&output);
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

/// The preorder walk of the tree without trivia and without trailing commas, with source offsets
fn steps(file: &SourceFile) -> Vec<(Step, TextSize)> {
    let mut out = Vec::new();
    for event in file.syntax().preorder_with_tokens() {
        match event {
            WalkEvent::Enter(NodeOrToken::Node(node)) => {
                out.push((Step::Enter(node.kind()), node.text_range().start()))
            }
            WalkEvent::Leave(NodeOrToken::Node(node)) => {
                out.push((Step::Leave(node.kind()), node.text_range().end()))
            }
            WalkEvent::Enter(NodeOrToken::Token(token))
                if !token.kind().is_trivia() && !is_trailing_comma(&token) =>
            {
                out.push((
                    Step::Token(token.kind(), token.text().to_string()),
                    token.text_range().start(),
                ))
            }
            _ => {}
        }
    }
    out
}

fn is_trailing_comma(token: &SyntaxToken) -> bool {
    token.kind() == SyntaxKind::COMMA
        && token
            .siblings_with_tokens(Direction::Next)
            .skip(1)
            .find(|e| !e.kind().is_trivia())
            .is_some_and(|e| {
                matches!(
                    e.kind(),
                    SyntaxKind::R_PAREN
                        | SyntaxKind::R_BRACK
                        | SyntaxKind::R_CURLY
                        | SyntaxKind::R_ANGLE
                )
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
    let before = &code[..usize::from(offset).min(code.len())];
    let line = before.matches('\n').count() + 1;
    let column = before.len() - before.rfind('\n').map_or(0, |i| i + 1) + 1;
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
        check_result(&file_a.tree(), &file_b.tree(), a, b)
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
}
