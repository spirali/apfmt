//! Rewrites of the source before it is formatted: changes of the code beyond its layout, e.g. sorting
//! imports. The result is parsed again and formatted like any other source.

mod imports;
mod sort;

use ra_ap_syntax::{SourceFile, TextRange};

/// The source after the rewrites, or `None` if none of them changes it
pub(crate) fn rewrite_source(file: &SourceFile, source: &str) -> Option<String> {
    apply_edits(source, imports::import_edits(file, source))
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
