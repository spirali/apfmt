//! Helpers that do not depend on the syntax tree (see `syntax_utils.rs` for those that do)

use std::path::{Path, PathBuf};

pub(crate) fn with_is_last<T>(items: &[T]) -> impl Iterator<Item = (&T, bool)> {
    items
        .iter()
        .enumerate()
        .map(|(idx, item)| (item, idx + 1 == items.len()))
}

/// Absolute, with symbolic links and `..` resolved as far as it can be
pub fn canonical_path(path: &Path) -> PathBuf {
    let path = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    path.canonicalize().unwrap_or(path)
}

pub(crate) fn file_in_dir(dir: &Path, names: &[&str]) -> Option<PathBuf> {
    names
        .iter()
        .map(|name| dir.join(name))
        .find(|path| path.is_file())
}

/// Printable ASCII, most text, is one column per byte
pub(crate) fn text_width(text: &str) -> u32 {
    if text.bytes().all(|b| (b' '..=b'~').contains(&b)) {
        return text.len() as u32;
    }
    unicode_width::UnicodeWidthStr::width(text) as u32
}
