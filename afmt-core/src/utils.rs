pub(crate) fn with_is_last<T>(items: &[T]) -> impl Iterator<Item = (&T, bool)> {
    items
        .iter()
        .enumerate()
        .map(|(idx, item)| (item, idx + 1 == items.len()))
}
