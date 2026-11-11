//! Removal of a `where` without predicates: `fn f() where {}`

use super::line_if_alone;
use crate::syntax_utils::{is_empty_where_clause, is_skipped};
use ra_ap_syntax::{AstNode, SourceFile, TextRange};

/// The edits that remove them, but in skipped code (`#[rustfmt::skip]`); a `where` alone on its line
/// is removed with its line
pub(super) fn empty_where_edits(file: &SourceFile, source: &str) -> Vec<(TextRange, String)> {
    file.syntax()
        .descendants()
        .filter(|node| is_empty_where_clause(node) && !is_skipped(node))
        .map(|node| (line_if_alone(source, node.text_range()), String::new()))
        .collect()
}
