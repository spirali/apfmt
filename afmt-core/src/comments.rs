//! Facts about comments and trivia in the source: token queries (what follows a token, where a
//! comment is); placing and normalizing comments in the output is `formatter/trivia.rs`

use ra_ap_syntax::{SyntaxKind, SyntaxNode, SyntaxToken};

/// The trivia tokens (whitespace, comments) starting at `token` (inclusive), up to the first code token;
/// `trivia_from(token.next_token())` is the trivia after `token`
pub(crate) fn trivia_from(token: Option<SyntaxToken>) -> impl Iterator<Item = SyntaxToken> {
    std::iter::successors(token, |t| t.next_token()).take_while(|t| t.kind().is_trivia())
}

/// The tokens from `first` to `last`, both included (`last` follows `first` in the source)
pub(crate) fn tokens_between(
    first: &SyntaxToken,
    last: &SyntaxToken,
) -> impl Iterator<Item = SyntaxToken> {
    let last = last.clone();
    std::iter::successors(Some(first.clone()), move |t| {
        (*t != last).then(|| t.next_token()).flatten()
    })
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
            matches!(
                t.kind(),
                SyntaxKind::L_CURLY | SyntaxKind::L_PAREN | SyntaxKind::L_BRACK
            )
        });
    token.kind() == SyntaxKind::COMMENT
        && token.text().starts_with("/*")
        && (line_start || after_opener)
        && !has_newline(token.next_token())
}

/// Whether the rest of the line after `token` holds only comments and the separator `kind` (if given),
/// i.e. no code follows on the same line after the separator
pub(crate) fn rest_of_line_is_comments(token: &SyntaxToken, separator: Option<SyntaxKind>) -> bool {
    let mut seen_separator = separator.is_none();
    let mut element = token.next_token();
    while let Some(t) = element {
        match t.kind() {
            SyntaxKind::WHITESPACE if seen_separator && count_newlines(&t) > 0 => return true,
            SyntaxKind::WHITESPACE | SyntaxKind::COMMENT => {}
            k if Some(k) == separator && !seen_separator => seen_separator = true,
            // Without the separator, the code is the end of the list, e.g. `)`
            _ => return !seen_separator,
        }
        element = t.next_token();
    }
    true
}

/// Whether only comments and the `separator` follow `token` up to the end of its line
pub(crate) fn line_ends_after(token: &SyntaxToken, separator: Option<SyntaxKind>) -> bool {
    std::iter::successors(token.next_token(), |t| t.next_token())
        .find(|t| match t.kind() {
            SyntaxKind::COMMENT => false,
            SyntaxKind::WHITESPACE => count_newlines(t) > 0,
            kind => Some(kind) != separator,
        })
        .is_none_or(|t| t.kind() == SyntaxKind::WHITESPACE)
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

/// Whether `node` contains a comment
pub(crate) fn has_comments(node: &SyntaxNode) -> bool {
    node.descendants_with_tokens()
        .any(|e| e.kind() == SyntaxKind::COMMENT)
}

pub(crate) fn is_line_comment(token: &SyntaxToken) -> bool {
    token.kind() == SyntaxKind::COMMENT && token.text().starts_with("//")
}

/// The comment `text` is a doc comment: `///` (not `////`), `//!`, `/**` (not `/***` nor the empty
/// `/**/`) or `/*!`
pub(crate) fn is_doc_comment(text: &str) -> bool {
    (text.starts_with("///") && !text.starts_with("////"))
        || text.starts_with("//!")
        || (text.starts_with("/**") && !text.starts_with("/***") && text != "/**/")
        || text.starts_with("/*!")
}

/// The first token of `node` that is not trivia; rust-analyzer attaches the comments in front of an item
/// to the item
pub(crate) fn first_code_token(node: &SyntaxNode) -> SyntaxToken {
    let first = node.first_token().unwrap();
    trivia_from(Some(first.clone()))
        .last()
        .map_or(Some(first), |t| t.next_token())
        .unwrap()
}

/// The whitespace at the start of the source line of `token`. The line starts after the last line
/// break in front of `token`, which is in a single token (a whitespace token, or a multi-line string
/// literal or comment), or at the start of the file
pub(crate) fn line_indentation(token: &SyntaxToken) -> String {
    let leading_whitespace = |text: &str| text[..text.len() - text.trim_start().len()].to_string();
    let mut line_start = None;
    for t in std::iter::successors(token.prev_token(), |t| t.prev_token()) {
        if let Some(pos) = t.text().rfind('\n') {
            return leading_whitespace(&t.text()[pos + 1..]);
        }
        line_start = Some(t);
    }
    line_start.map_or_else(String::new, |t| leading_whitespace(t.text()))
}

/// The first token after `token` that is not trivia
pub(crate) fn next_code_token(token: &SyntaxToken) -> Option<SyntaxToken> {
    std::iter::successors(token.next_token(), |t| t.next_token()).find(|t| !t.kind().is_trivia())
}

/// The first token before `token` that is not trivia
pub(crate) fn prev_code_token(token: &SyntaxToken) -> Option<SyntaxToken> {
    std::iter::successors(token.prev_token(), |t| t.prev_token()).find(|t| !t.kind().is_trivia())
}

/// Where taking the trivia after a token stops at the latest (see `trivia_end`)
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TriviaStop {
    /// In front of the first token that is not trivia
    Code,
    /// In front of the first own-line comment
    OwnLine,
    /// Right after the separator; comments behind it are left
    Separator,
}

/// The last token of the trivia taken after `token`: comments up to `stop`, and the `separator` if it
/// follows them (comments in front of a separator are never own-line, they move behind it); `token`
/// itself when nothing is taken. `Builder::push_trivia` places exactly these tokens.
pub(crate) fn trivia_end(
    token: &SyntaxToken,
    separator: Option<SyntaxKind>,
    stop: TriviaStop,
) -> SyntaxToken {
    let next_code = next_code_token(token).map(|t| t.kind());
    let mut separator = separator.filter(|kind| next_code == Some(*kind));
    let first = token.next_token();
    let mut end = token.clone();
    let mut newlines = 0;
    for t in std::iter::successors(first, |t| t.next_token()) {
        match t.kind() {
            SyntaxKind::WHITESPACE => newlines += count_newlines(&t),
            SyntaxKind::COMMENT => {
                if separator.is_some() {
                    newlines = 0;
                }
                if newlines > 0 && stop != TriviaStop::Code {
                    break;
                }
                newlines = 0;
                end = t;
            }
            kind if Some(kind) == separator => {
                newlines = 0;
                separator = None;
                end = t;
                if stop == TriviaStop::Separator {
                    break;
                }
            }
            _ => break,
        }
    }
    end
}

/// Where the post comments of a list item ending with `token` stop (as rustfmt calls them): after the
/// comments ending the line of the `separator` (an own-line comment belongs to the next item), or
/// right after the separator if code follows on its line
pub(crate) fn post_comments_stop(token: &SyntaxToken, separator: SyntaxKind) -> TriviaStop {
    if rest_of_line_is_comments(token, Some(separator)) {
        TriviaStop::OwnLine
    } else {
        TriviaStop::Separator
    }
}

/// The last token of the post comments of a list item ending with `token`, its separator included
/// (see `post_comments_stop`); what `Builder::push_separator_with_trailing_comments` takes
pub(crate) fn post_comments_end(token: &SyntaxToken, separator: SyntaxKind) -> SyntaxToken {
    let stop = post_comments_stop(token, separator);
    trivia_end(token, Some(separator), stop)
}

/// The last token of the comments ending the line of `token` (`( // c`), or `token` if code follows
/// on its line; what `Builder::push_comments_ending_line` takes
pub(crate) fn comments_ending_line_end(token: &SyntaxToken) -> SyntaxToken {
    if rest_of_line_is_comments(token, None) {
        trivia_end(token, None, TriviaStop::OwnLine)
    } else {
        token.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::is_doc_comment;

    #[test]
    fn doc_comments() {
        for (comment, doc) in [
            ("/// a", true),
            ("//! a", true),
            ("/** a */", true),
            ("/*! a */", true),
            ("// a", false),
            ("//// a", false),
            ("/* a */", false),
            ("/**/", false),
            ("/*** a */", false),
        ] {
            assert_eq!(is_doc_comment(comment), doc, "{comment}");
        }
    }
}
