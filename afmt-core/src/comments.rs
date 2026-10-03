use ra_ap_syntax::SyntaxToken;

/// Puts a single space after the opening and before the closing sigil of a single-line
/// block comment (`/*a*/` -> `/* a */`). Returns `None` when the text stays as it is.
/// Doc comments and empty comments are left alone.
pub(crate) fn normalize_block_comment(text: &str) -> Option<String> {
    let inner = text.strip_prefix("/*")?.strip_suffix("*/")?;
    if inner.contains('\n') || inner.starts_with('*') || inner.starts_with('!') {
        return None;
    }
    let trimmed = inner.trim();
    if trimmed.is_empty() {
        return None;
    }
    let normalized = format!("/* {trimmed} */");
    (normalized != text).then_some(normalized)
}

fn source_column(token: &SyntaxToken) -> usize {
    let mut col = 0;
    let mut element = token.prev_token();
    while let Some(token) = element {
        let text = token.text();
        if let Some(pos) = text.rfind('\n') {
            return col + unicode_width::UnicodeWidthStr::width(&text[pos + 1..]);
        }
        col += unicode_width::UnicodeWidthStr::width(text);
        element = token.prev_token();
    }
    col
}

/// Lines of a multi-line block comment with the indentation of the comment start removed, so they can
/// be aligned to the new column of the comment; blank lines are empty. `None` when the comment is
/// single-line or a line is indented less than the comment start.
pub(crate) fn multiline_comment_lines(token: &SyntaxToken) -> Option<Vec<&str>> {
    let (first, rest) = token.text().split_once('\n')?;
    let col = source_column(token);
    let is_blank = |line: &str| line.trim().is_empty();
    if !rest.split('\n').all(|line| {
        is_blank(line) || (line.len() >= col && line.as_bytes()[..col].iter().all(|c| *c == b' '))
    }) {
        return None;
    }
    let rest = rest.split('\n').map(|line| if is_blank(line) { "" } else { &line[col..] });
    Some(std::iter::once(first).chain(rest).collect())
}
