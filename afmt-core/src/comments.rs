//! Facts about comments and trivia in the source: token queries (what follows a token, where a
//! comment is) and comment text; placing comments in the output is `formatter/trivia.rs`

use ra_ap_syntax::{SyntaxKind, SyntaxToken};

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
    let rest = rest
        .split('\n')
        .map(|line| if is_blank(line) { "" } else { &line[col..] });
    Some(std::iter::once(first).chain(rest).collect())
}

/// The trivia tokens (whitespace, comments) starting at `token` (inclusive), up to the first code token;
/// `trivia_from(token.next_token())` is the trivia after `token`
pub(crate) fn trivia_from(token: Option<SyntaxToken>) -> impl Iterator<Item = SyntaxToken> {
    std::iter::successors(token, |t| t.next_token()).take_while(|t| t.kind().is_trivia())
}

pub(crate) fn count_newlines(token: &SyntaxToken) -> u32 {
    token.text().bytes().filter(|c| *c == b'\n').count() as u32
}

/// A block comment at the start of a line or right after an opening delimiter, directly followed by
/// code on the same line, e.g. `/* c */ f();` or `{ /* c */ 1 }`: it belongs to that code
pub(crate) fn is_leading_block_comment(token: &SyntaxToken) -> bool {
    let has_newline = |t: Option<SyntaxToken>| {
        t.is_some_and(|t| t.kind() == SyntaxKind::WHITESPACE && count_newlines(&t) > 0)
    };
    // The start of the source is the start of a line too
    let line_start = token.prev_token().is_none() || has_newline(token.prev_token());
    let after_opener = std::iter::successors(token.prev_token(), |t| t.prev_token())
        .find(|t| !t.kind().is_trivia())
        .is_some_and(|t| {
            matches!(t.kind(), SyntaxKind::L_CURLY | SyntaxKind::L_PAREN | SyntaxKind::L_BRACK)
        });
    token.kind() == SyntaxKind::COMMENT
        && token.text().starts_with("/*")
        && (line_start || after_opener)
        && !has_newline(token.next_token())
}

/// Whether the rest of the line after `token` holds only comments and the delimiter `kind` (if given),
/// i.e. no code follows on the same line after the delimiter
pub(crate) fn rest_of_line_is_comments(token: &SyntaxToken, delimiter: Option<SyntaxKind>) -> bool {
    let mut seen_delimiter = delimiter.is_none();
    let mut element = token.next_token();
    while let Some(t) = element {
        match t.kind() {
            SyntaxKind::WHITESPACE if seen_delimiter && count_newlines(&t) > 0 => return true,
            SyntaxKind::WHITESPACE | SyntaxKind::COMMENT => {}
            k if Some(k) == delimiter && !seen_delimiter => seen_delimiter = true,
            // Without the delimiter, the code is the end of the list, e.g. `)`
            _ => return !seen_delimiter,
        }
        element = t.next_token();
    }
    true
}

/// Whether an own-line comment (one on a later line) follows `token` before the next code token
pub(crate) fn own_line_comment_follows(token: &SyntaxToken) -> bool {
    let mut newlines = 0;
    trivia_from(token.next_token()).any(|t| {
        if t.kind() == SyntaxKind::WHITESPACE {
            newlines += count_newlines(&t);
        }
        t.kind() == SyntaxKind::COMMENT && newlines > 0
    })
}
