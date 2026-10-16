//! Removal of empty statements: the stray `;` in a block, `;;` or `g();;` (as in rustfmt)

use ra_ap_syntax::{AstNode, SourceFile, SyntaxKind, TextRange, TextSize};

/// The edits that remove the stray `;` of the blocks of the file, which are not part of a statement;
/// a `;` alone on its line is removed with its line, which would otherwise become a blank line
pub(super) fn empty_stmt_edits(file: &SourceFile, source: &str) -> Vec<(TextRange, String)> {
    file.syntax()
        .descendants()
        .filter(|node| node.kind() == SyntaxKind::STMT_LIST)
        .flat_map(|list| list.children_with_tokens())
        .filter_map(|element| element.into_token())
        .filter(|token| token.kind() == SyntaxKind::SEMICOLON)
        .map(|token| (line_if_alone(source, token.text_range()), String::new()))
        .collect()
}

/// The whole line of `range` with its line break when only spaces and tabs surround `range` on it
fn line_if_alone(source: &str, range: TextRange) -> TextRange {
    let blank = [' ', '\t'];
    let before = &source[..usize::from(range.start())];
    let indent = before.len() - before.trim_end_matches(blank).len();
    let after = &source[usize::from(range.end())..];
    let rest = after.len() - after.trim_start_matches(blank).len();
    let line_start = before.len() - indent;
    let line_end = usize::from(range.end()) + rest;
    let alone = (line_start == 0 || before[..line_start].ends_with('\n'))
        && source[line_end..].starts_with('\n');
    if !alone {
        return range;
    }
    let size = |n: usize| TextSize::from(n as u32);
    TextRange::new(size(line_start), size(line_end + 1))
}
