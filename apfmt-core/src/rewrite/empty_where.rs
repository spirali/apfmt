//! Removal of a `where` without predicates: `fn f() where {}`

use super::line_if_alone;
use crate::comments::comment_follows;
use crate::syntax_utils::{is_empty_where_clause, is_skipped};
use ra_ap_syntax::{AstNode, SourceFile, TextRange, ast};

/// The edits that remove them, but in skipped code (`#[rustfmt::skip]`); a `where` alone on its line
/// is removed with its line. Only the keyword goes when a comment follows it.
pub(super) fn empty_where_edits(file: &SourceFile, source: &str) -> Vec<(TextRange, String)> {
    file.syntax()
        .descendants()
        .filter(|node| is_empty_where_clause(node) && !is_skipped(node))
        .map(|node| {
            let range = match ast::WhereClause::cast(node.clone()).and_then(|c| c.where_token()) {
                Some(token) if comment_follows(&token) => token.text_range(),
                _ => node.text_range(),
            };
            (line_if_alone(source, range), String::new())
        })
        .collect()
}
