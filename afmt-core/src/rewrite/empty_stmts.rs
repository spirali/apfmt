//! Removal of empty statements: the stray `;` in a block, `;;` or `g();;` (as in rustfmt)

use super::line_if_alone;
use crate::syntax_utils::is_stray_semicolon;
use ra_ap_syntax::{AstNode, SourceFile, TextRange};

/// The edits that remove the stray `;` of blocks; a `;` alone on its line is removed with its line
pub(super) fn empty_stmt_edits(file: &SourceFile, source: &str) -> Vec<(TextRange, String)> {
    file.syntax()
        .descendants_with_tokens()
        .filter_map(|element| element.into_token())
        .filter(is_stray_semicolon)
        .map(|token| (line_if_alone(source, token.text_range()), String::new()))
        .collect()
}
