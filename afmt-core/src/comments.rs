//! Facts about comments and trivia in the source: token queries (what follows a token, where a
//! comment is); placing and normalizing comments in the output is `formatter/trivia.rs`

use ra_ap_syntax::{AstNode, SyntaxKind, SyntaxNode, SyntaxToken, ast};

/// `//`, `/* */` or a doc comment; the parser puts a doc comment in a `DOC_COMMENT` node, for afmt
/// it is a comment like any other
pub(crate) fn is_comment_kind(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::COMMENT
            | SyntaxKind::INNER_DOC_COMMENT
            | SyntaxKind::OUTER_DOC_COMMENT
            | SyntaxKind::DOC_COMMENT
    )
}

/// `///` or `/** */` documenting the next item, field, statement, ...: it starts a line, as if a
/// line break preceded it. On an expression only when it ends its line (`/** a */ 1` stays), as
/// in rustfmt.
pub(crate) fn is_outer_doc_comment(token: &SyntaxToken) -> bool {
    let on_expr = || {
        token
            .parent()
            .and_then(|doc| doc.parent())
            .is_some_and(|node| ast::Expr::can_cast(node.kind()))
    };
    token.kind() == SyntaxKind::OUTER_DOC_COMMENT
        && (rest_of_line_is_comments(token, None) || !on_expr())
}

/// Neither whitespace nor a comment; unlike `SyntaxKind::is_trivia`, doc comments are trivia
pub(crate) fn is_code_kind(kind: SyntaxKind) -> bool {
    kind != SyntaxKind::WHITESPACE && !is_comment_kind(kind)
}

/// Starting at `token` (inclusive), up to the first code token
pub(crate) fn trivia_from(token: Option<SyntaxToken>) -> impl Iterator<Item = SyntaxToken> {
    std::iter::successors(token, |t| t.next_token()).take_while(|t| !is_code_kind(t.kind()))
}

/// Both included
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

/// The newlines in the whitespace right after `token`
pub(crate) fn newlines_after(token: &SyntaxToken) -> u32 {
    trivia_from(token.next_token())
        .take_while(|t| t.kind() == SyntaxKind::WHITESPACE)
        .map(|t| count_newlines(&t))
        .sum()
}

/// A comment between `token` and the next code
pub(crate) fn comment_follows(token: &SyntaxToken) -> bool {
    trivia_from(token.next_token()).any(|t| is_comment_kind(t.kind()))
}

/// At the start of a line (after block comments only) or after an opening delimiter, followed by
/// code: `/* c */ f();`, `/* a */ /* b */ f();`, `{ /* c */ 1 }`
pub(crate) fn is_leading_block_comment(token: &SyntaxToken) -> bool {
    let has_newline = |t: Option<SyntaxToken>| {
        t.is_some_and(|t| t.kind() == SyntaxKind::WHITESPACE && count_newlines(&t) > 0)
    };
    let line_start = std::iter::successors(token.prev_token(), |t| t.prev_token())
        .find(|t| match t.kind() {
            SyntaxKind::WHITESPACE => count_newlines(t) > 0,
            kind => !is_comment_kind(kind) || is_line_comment(t),
        })
        .is_none_or(|t| has_newline(Some(t)));
    let after_opener = std::iter::successors(token.prev_token(), |t| t.prev_token())
        .find(|t| is_code_kind(t.kind()))
        .is_some_and(|t| {
            matches!(
                t.kind(),
                SyntaxKind::L_CURLY | SyntaxKind::L_PAREN | SyntaxKind::L_BRACK
            )
        });
    is_comment_kind(token.kind())
        && token.text().starts_with("/*")
        && (line_start || after_opener)
        && !has_newline(token.next_token())
}

/// No code follows on the same line after the separator `kind` (if given)
pub(crate) fn rest_of_line_is_comments(token: &SyntaxToken, separator: Option<SyntaxKind>) -> bool {
    let mut seen_separator = separator.is_none();
    let mut element = token.next_token();
    while let Some(t) = element {
        match t.kind() {
            SyntaxKind::WHITESPACE if seen_separator && count_newlines(&t) > 0 => return true,
            _ if seen_separator && is_outer_doc_comment(&t) => return true,
            k if is_comment_kind(k) => {}
            SyntaxKind::WHITESPACE => {}
            k if Some(k) == separator && !seen_separator => seen_separator = true,
            // Without the separator, the code is the end of the list, e.g. `)`
            _ => return !seen_separator,
        }
        element = t.next_token();
    }
    true
}

pub(crate) fn line_ends_after(token: &SyntaxToken, separator: Option<SyntaxKind>) -> bool {
    std::iter::successors(token.next_token(), |t| t.next_token())
        .find(|t| match t.kind() {
            _ if is_outer_doc_comment(t) => true,
            kind if is_comment_kind(kind) => false,
            SyntaxKind::WHITESPACE => count_newlines(t) > 0,
            kind => Some(kind) != separator,
        })
        .is_none_or(|t| t.kind() == SyntaxKind::WHITESPACE || is_outer_doc_comment(&t))
}

/// The first comment on a later line than `token`, before the next code token. Consecutive
/// whitespace is a single token, so the token after the first line break is the comment or code.
pub(crate) fn first_own_line_comment(token: &SyntaxToken) -> Option<SyntaxToken> {
    let line_break = trivia_from(token.next_token()).find(|t| {
        (t.kind() == SyntaxKind::WHITESPACE && t.text().contains('\n')) || is_outer_doc_comment(t)
    })?;
    if is_outer_doc_comment(&line_break) {
        return Some(line_break);
    }
    line_break
        .next_token()
        .filter(|t| is_comment_kind(t.kind()))
}

pub(crate) fn own_line_comment_follows(token: &SyntaxToken) -> bool {
    first_own_line_comment(token).is_some()
}

/// An own-line comment, or a line comment ending the line of `token`
pub(crate) fn comments_break_line_after(token: &SyntaxToken) -> bool {
    trivia_from(token.next_token()).any(|t| is_line_comment(&t)) || own_line_comment_follows(token)
}

pub(crate) fn has_comments(node: &SyntaxNode) -> bool {
    node.descendants_with_tokens()
        .any(|e| is_comment_kind(e.kind()))
}

pub(crate) fn is_line_comment(token: &SyntaxToken) -> bool {
    is_comment_kind(token.kind()) && token.text().starts_with("//")
}

/// `///` (not `////`), `//!`, `/**` (not `/***` nor `/**/`) or `/*!`
pub(crate) fn is_doc_comment(text: &str) -> bool {
    (text.starts_with("///") && !text.starts_with("////"))
        || text.starts_with("//!")
        || (text.starts_with("/**") && !text.starts_with("/***") && text != "/**/")
        || text.starts_with("/*!")
}

/// rust-analyzer attaches the comments in front of an item to the item
pub(crate) fn first_code_token(node: &SyntaxNode) -> SyntaxToken {
    let first = node.first_token().unwrap();
    trivia_from(Some(first.clone()))
        .last()
        .map_or(Some(first), |t| t.next_token())
        .unwrap()
}

/// The last line break in front of `token` is in a single token, or it is the start of the file
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

/// Own-line comments in front of `close` at most at its indentation: the first one decides for all,
/// as in rustfmt (`// Safety: ...` / `} else if ...`)
pub(crate) fn comments_at_closing_indentation(close: &SyntaxToken) -> bool {
    let comment = prev_code_token(close).and_then(|t| first_own_line_comment(&t));
    comment.is_some_and(|c| line_indentation(&c).len() <= line_indentation(close).len())
}

pub(crate) fn next_code_token(token: &SyntaxToken) -> Option<SyntaxToken> {
    std::iter::successors(token.next_token(), |t| t.next_token()).find(|t| is_code_kind(t.kind()))
}

pub(crate) fn prev_code_token(token: &SyntaxToken) -> Option<SyntaxToken> {
    std::iter::successors(token.prev_token(), |t| t.prev_token()).find(|t| is_code_kind(t.kind()))
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TriviaStop {
    Code,
    OwnLine,
    /// Right after the separator; comments behind it, or behind its place if the source has none,
    /// are left
    Separator,
}

/// The last token of the trivia taken after `token`: comments up to `stop`, and the `separator` if
/// it follows them. `Builder::push_trivia` places exactly these tokens.
pub(crate) fn trivia_end(
    token: &SyntaxToken,
    separator: Option<SyntaxKind>,
    stop: TriviaStop,
) -> SyntaxToken {
    // The separator is taken only as the first code token after `token`
    let mut separator = separator;
    let mut separator_follows = None;
    let first = token.next_token();
    let mut end = token.clone();
    let mut newlines = 0;
    for t in std::iter::successors(first, |t| t.next_token()) {
        match t.kind() {
            SyntaxKind::WHITESPACE => newlines += count_newlines(&t),
            kind if is_comment_kind(kind) => {
                let follows = *separator_follows.get_or_insert_with(|| {
                    separator
                        .is_some_and(|sep| next_code_token(&t).is_some_and(|c| c.kind() == sep))
                });
                if follows && separator.is_some() {
                    newlines = 0;
                }
                // Without the separator in the source, the comments are behind its place
                if !follows && separator.is_some() && stop == TriviaStop::Separator {
                    break;
                }
                let starts_line = newlines > 0 || (is_outer_doc_comment(&t) && separator.is_none());
                if starts_line && stop != TriviaStop::Code {
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

/// After the comments ending the line of the separator (an own-line comment belongs to the next
/// item), or right after it if code follows (rustfmt's post comments)
pub(crate) fn post_comments_stop(token: &SyntaxToken, separator: SyntaxKind) -> TriviaStop {
    if rest_of_line_is_comments(token, Some(separator)) {
        TriviaStop::OwnLine
    } else {
        TriviaStop::Separator
    }
}

/// Including the separator; what `Builder::push_separator_with_trailing_comments` takes
pub(crate) fn post_comments_end(token: &SyntaxToken, separator: SyntaxKind) -> SyntaxToken {
    let stop = post_comments_stop(token, separator);
    trivia_end(token, Some(separator), stop)
}

/// `token` if code follows on its line; what `Builder::push_comments_ending_line` takes
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
