//! Rewrites of the source before it is formatted: changes of the code beyond its layout, e.g. sorting
//! imports. The result is parsed again and formatted like any other source.

mod empty_stmts;
mod imports;
mod sort;

use ra_ap_syntax::{SourceFile, TextRange};

/// The source after the rewrites, or `None` if none of them changes it
pub(crate) fn rewrite_source(file: &SourceFile, source: &str) -> Option<String> {
    let mut edits = imports::import_edits(file, source);
    // A stray `;` between imports is dropped by the edit of their group (`use a; ; use b;`)
    let in_imports = |range: &TextRange| edits.iter().any(|(r, _)| r.contains_range(*range));
    let empty_stmts: Vec<_> = empty_stmts::empty_stmt_edits(file, source)
        .into_iter()
        .filter(|(range, _)| !in_imports(range))
        .collect();
    edits.extend(empty_stmts);
    apply_edits(source, edits)
}

/// The source with each range replaced by its text (the ranges do not overlap); `None` without edits
fn apply_edits(source: &str, mut edits: Vec<(TextRange, String)>) -> Option<String> {
    if edits.is_empty() {
        return None;
    }
    edits.sort_by_key(|(range, _)| range.start());
    let mut out = String::with_capacity(source.len());
    let mut pos = 0;
    for (range, text) in edits {
        out.push_str(&source[pos..usize::from(range.start())]);
        out.push_str(&text);
        pos = range.end().into();
    }
    out.push_str(&source[pos..]);
    Some(out)
}
