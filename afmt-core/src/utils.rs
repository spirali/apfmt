use ra_ap_syntax::TextSize;

pub(crate) fn with_is_last<T>(items: &[T]) -> impl Iterator<Item = (&T, bool)> {
    items
        .iter()
        .enumerate()
        .map(|(idx, item)| (item, idx + 1 == items.len()))
}

/// The line and the column (both from 1, the column in characters) of `offset` in `code`
pub(crate) fn line_column(code: &str, offset: TextSize) -> (usize, usize) {
    let before = &code[..usize::from(offset).min(code.len())];
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    (
        before.matches('\n').count() + 1,
        before[line_start..].chars().count() + 1,
    )
}
