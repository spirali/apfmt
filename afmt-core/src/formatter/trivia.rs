use super::frag::{Edge, Frag};
use super::{Builder, Formatter};
use crate::comments::{
    TriviaStop, count_newlines, is_comment_kind, is_leading_block_comment, is_line_comment,
    line_ends_after, line_indentation, next_code_token, post_comments_stop,
    rest_of_line_is_comments, tokens_between, trivia_end, trivia_from,
};
use crate::doc::{COMMA_DOC, DocId, EMPTY_DOC, EMPTYLINE_DOC, NEWLINE_DOC, SEMICOLON_DOC};
use crate::formatter::Edge::{Break, Comma, Nothing, Word};
use ra_ap_syntax::{SyntaxKind, SyntaxToken};

fn separator_doc(kind: SyntaxKind) -> DocId {
    match kind {
        SyntaxKind::SEMICOLON => SEMICOLON_DOC,
        SyntaxKind::COMMA => COMMA_DOC,
        _ => unreachable!(),
    }
}

/// `//a` -> `// a`, `/*a*/` -> `/* a */` when the text starts with a letter or a digit (see
/// CLAUDE.md); `None` when the text stays. Not for doc comments, whose text is Markdown.
pub(super) fn normalize_comment_sigil_spacing(text: &str) -> Option<String> {
    let starts_with_alphanumeric = |s: &str| s.starts_with(char::is_alphanumeric);
    if let Some(rest) = text.strip_prefix("//") {
        return starts_with_alphanumeric(rest).then(|| format!("// {rest}"));
    }
    let inner = text.strip_prefix("/*")?.strip_suffix("*/")?;
    let trimmed = inner.trim();
    let normalized =
        inner.len() == trimmed.len() + 2 && inner.starts_with(' ') && inner.ends_with(' ');
    (!normalized && !inner.contains('\n') && starts_with_alphanumeric(trimmed))
        .then(|| format!("/* {trimmed} */"))
}

/// Without trailing whitespace; a doc comment (`doc`) keeps it, two trailing spaces end a Markdown
/// line
pub(super) fn trim_comment_line(line: &str, doc: bool) -> &str {
    if doc { line } else { line.trim_end() }
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

/// Lines of a multi-line block comment without the indentation of the comment start. `None` when
/// single-line or a line is indented less than the comment start.
pub(super) fn multiline_comment_lines(token: &SyntaxToken) -> Option<Vec<&str>> {
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

impl<'a> Builder<'a> {
    /// A trailing line comment would otherwise swallow the following code
    pub fn end_line_after_line_comment(&mut self) {
        self.push_line_break_if(NEWLINE_DOC, |end| end == Break);
    }

    /// Pushes the trivia after the cursor and ends the line; not after a block comment belonging to
    /// the next code (`/* c */ f();`)
    pub fn flush_trivia_and_end_line(&mut self) {
        self.flush_trivia_and_end_line_with(true);
    }

    /// Ends the line of a list item; block comments followed by the next item on their line stay
    /// pending, in front of it (`a, /* b */ b,` → `a,` / `/* b */ b,`)
    pub fn end_line_before_item(&mut self, keep_blank_lines: bool) {
        if self
            .cursor()
            .is_some_and(|c| !rest_of_line_is_comments(c, None))
        {
            self.push_newline();
        } else {
            self.flush_trivia_and_end_line_with(keep_blank_lines);
        }
    }

    /// As `flush_trivia_and_end_line`, with the blank lines after the last comment dropped unless
    /// `keep_blank_lines`
    pub fn flush_trivia_and_end_line_with(&mut self, keep_blank_lines: bool) {
        self.push_comments();
        let leading_comment = self
            .frag
            .cursor
            .as_ref()
            .is_some_and(is_leading_block_comment);
        if keep_blank_lines {
            self.flush_trivia_keeping_blank_lines();
        } else {
            self.flush_trivia();
        }
        if leading_comment {
            return;
        }
        if self.frag.is_empty() {
            self.push_newline();
        } else {
            self.push_line_break_if(NEWLINE_DOC, |end| end != Nothing);
        }
    }

    /*fn push_emptyline(&mut self) {
        self.push_line_break_if(EMPTYLINE_DOC, |_| true);
        //self.frag.empty_lines = 0;
    }*/

    /// At most `blank_lines_upper_bound`; a single newline only ends a line comment
    fn push_blank_lines(&mut self, newlines: u32) {
        if newlines == 0 {
            return;
        }
        let mut blank = (newlines - 1).min(self.fmt.config.blank_lines_upper_bound);
        if blank == 0 {
            self.end_line_after_line_comment();
            return;
        }
        let mut doc_id = EMPTY_DOC;
        blank += 1;
        while blank > 1 {
            doc_id = self.fmt.concat(doc_id, EMPTYLINE_DOC);
            blank -= 2;
        }
        if blank == 1 {
            doc_id = self.fmt.concat(doc_id, NEWLINE_DOC);
        }
        self.push_line_break_if(doc_id, |_| true);
    }

    /// Pushes the comments after the cursor and moves the cursor to the last of them; the
    /// whitespace after it is left for `flush_trivia`
    pub fn push_comments(&mut self) {
        self.push_trivia(None, TriviaStop::Code);
    }

    /// A line comment ends the line. A `trailing` comment does not count as overflow, so it does
    /// not change the layout of the code in front of it, as in rustfmt.
    fn push_comment(&mut self, token: &SyntaxToken, trailing: bool) {
        let end = if is_line_comment(token) { Break } else { Word };
        let mut doc_id = self.fmt.comment_doc(token);
        if trailing && !token.text().contains('\n') {
            if let Some(group) = self.align_group {
                doc_id = self.fmt.concat(self.fmt.align_mark(group), doc_id);
            }
            doc_id = self.fmt.ignore_overflow(doc_id);
        }
        self.push(Frag::new(doc_id, Edge::Comment, end, None));
    }

    /// For trivia that no cursor precedes, e.g. at the start of the source; they start on a new
    /// line and the last comment becomes the cursor
    pub fn push_comments_from(&mut self, token: Option<SyntaxToken>) {
        let first_comment = trivia_from(token).find(|t| is_comment_kind(t.kind()));
        if let Some(comment) = first_comment {
            self.push_line_break_if(NEWLINE_DOC, |end| end != Nothing);
            self.push_comment(&comment, false);
            self.frag.cursor = Some(comment);
            self.push_comments();
        }
    }

    /// Pushes the comments after the cursor up to `stop` (see `trivia_end`), taking the source
    /// `separator` after them without printing. Returns whether a comment was pushed.
    pub(super) fn push_trivia(&mut self, separator: Option<SyntaxKind>, stop: TriviaStop) -> bool {
        let Some(cursor) = self.frag.cursor.take() else {
            return false;
        };
        let end = trivia_end(&cursor, separator, stop);
        if end == cursor {
            self.frag.cursor = Some(end);
            return false;
        }
        let mut pushed = false;
        let mut newlines = 0;
        // Comments in front of the separator are moved behind it
        let mut before_separator = separator
            .is_some_and(|kind| next_code_token(&cursor).is_some_and(|t| t.kind() == kind));
        let mut element = cursor.next_token();
        while cursor != end
            && let Some(token) = element
        {
            match token.kind() {
                SyntaxKind::WHITESPACE => newlines += count_newlines(&token),
                kind if is_comment_kind(kind) => {
                    if before_separator {
                        newlines = 0;
                    }
                    // Blank lines at the start of a fragment are dropped; the parent decides
                    // spacing
                    if newlines > 0 && !self.frag.is_empty() {
                        self.push_blank_lines(newlines);
                    }
                    let trailing = newlines == 0
                        && !pushed
                        && !self.frag.is_empty()
                        && line_ends_after(&token, separator.filter(|_| before_separator));
                    if newlines > 0 {
                        self.push_line_break_if(NEWLINE_DOC, |end| end != Nothing);
                    }
                    newlines = 0;
                    self.push_comment(&token, trailing);
                    pushed = true;
                }
                _ => {
                    newlines = 0;
                    before_separator = false;
                }
            }
            if token == end {
                break;
            }
            element = token.next_token();
        }
        self.frag.cursor = Some(end);
        pushed
    }

    /// Pushes the comments after the cursor, then clears it; blank lines after the last comment are
    /// dropped
    pub fn flush_trivia(&mut self) {
        self.push_comments();
        self.frag.cursor = None;
    }

    /// E.g. between statements
    fn flush_trivia_keeping_blank_lines(&mut self) {
        self.push_comments();
        let Some(cursor) = self.frag.cursor.take() else {
            return;
        };
        let newlines: u32 = trivia_from(cursor.next_token())
            .take_while(|t| t.kind() == SyntaxKind::WHITESPACE)
            .map(|t| count_newlines(&t))
            .sum();
        if newlines > 1 {
            self.push_blank_lines(newlines);
        }
    }

    /// Pushes the separator `kind` and takes the source one with the comments in front of it, which
    /// go after it. The cursor is then the separator.
    pub fn push_separator(&mut self, kind: SyntaxKind) {
        self.take_separator(kind, true);
    }

    pub fn skip_separator(&mut self, kind: SyntaxKind) {
        self.take_separator(kind, false);
    }

    pub fn push_or_skip_separator(&mut self, kind: SyntaxKind, push: bool) {
        self.take_separator(kind, push);
    }

    /// After a repetition of `macro_rules!`, a separator is pushed only where the source has one
    /// (`$($id,)+,` would not compile)
    fn take_separator(&mut self, kind: SyntaxKind, push: bool) {
        // The trivia after the cursor follows the separator
        let cursor = self.frag.cursor.take();
        let push = push
            && cursor.as_ref().is_none_or(|cursor| {
                !self.fmt.is_repetition_placeholder(cursor)
                    || next_code_token(cursor).is_some_and(|t| t.kind() == kind)
            });
        if push {
            self.push(Frag::new_simple(separator_doc(kind), Comma, None));
        }
        self.frag.cursor = cursor;
        self.push_trivia(Some(kind), TriviaStop::Separator);
    }

    /// Takes the source separator with the item's post comments (rustfmt): those in front of it and
    /// those ending its line. Returns whether a post comment was pushed.
    pub fn push_separator_with_trailing_comments(&mut self, kind: SyntaxKind) -> bool {
        // The trivia after the cursor follows the separator
        let cursor = self.frag.cursor.take();
        self.push(Frag::new_simple(separator_doc(kind), Comma, None));
        let Some(cursor) = cursor else {
            return false;
        };
        let stop = post_comments_stop(&cursor, kind);
        self.frag.cursor = Some(cursor);
        self.push_trivia(Some(kind), stop)
    }

    /// rustfmt's pre comments; a line break follows them when the source has one
    pub fn push_leading_comments(&mut self) {
        self.push_comments();
        let Some(cursor) = self.frag.cursor.as_ref() else {
            return;
        };
        if is_comment_kind(cursor.kind())
            && cursor
                .next_token()
                .is_some_and(|t| t.kind() == SyntaxKind::WHITESPACE && count_newlines(&t) > 0)
        {
            self.push_line_break_if(NEWLINE_DOC, |end| end != Nothing);
        }
    }

    /// Pushes the comments after the cursor if nothing else follows on its line (`( // c`); returns
    /// whether it pushed any. Placed with `Formatter::body_after_opener`.
    pub fn push_comments_ending_line(&mut self) -> bool {
        let ends_line = self
            .frag
            .cursor
            .as_ref()
            .is_some_and(|c| rest_of_line_is_comments(c, None));
        ends_line && self.push_trivia(None, TriviaStop::OwnLine)
    }
}

impl Formatter<'_> {
    /// The source from `first` to `last` as lines, without the indentation of the line of `first`
    /// (`None` when a line is indented less or with a tab); placeholders written as their text
    pub(super) fn reindented_lines(
        &self,
        first: &SyntaxToken,
        last: &SyntaxToken,
    ) -> Option<Vec<String>> {
        let base = line_indentation(first);
        if base.contains(|c| c != ' ') {
            return None;
        }
        let base = base.len();
        let mut lines = vec![String::new()];
        for token in tokens_between(first, last) {
            let text = token.text();
            match text.rfind('\n') {
                Some(pos) if token.kind() == SyntaxKind::WHITESPACE => {
                    let indent = &text[pos + 1..];
                    if indent.len() < base || indent.contains(|c| c != ' ') {
                        return None;
                    }
                    lines.extend((1..count_newlines(&token)).map(|_| String::new()));
                    lines.push(indent[base..].to_string());
                }
                _ => lines.last_mut().unwrap().push_str(&self.token_text(&token)),
            }
        }
        Some(lines)
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_comment_sigil_spacing;

    #[test]
    fn comment_sigil_spacing() {
        for (comment, expected) in [
            ("//a", Some("// a")),
            ("//let x = 1;", Some("// let x = 1;")),
            ("// a", None),
            ("//TODO", Some("// TODO")),
            ("//é", Some("// é")),
            // empty comments
            ("//", None),
            ("/**/", None),
            ("/*  */", None),
            // doc comments and `////`
            ("///a", None),
            ("//!a", None),
            ("////a", None),
            ("/**a*/", None),
            ("/*!a*/", None),
            // no letter or digit after the sigil: whitespace, separators, commented-out code
            ("//\ta", None),
            ("//-----", None),
            ("//#[derive(Debug)]", None),
            ("//(note) a", None),
            ("/*---*/", None),
            ("/*(x)*/", None),
            ("/*a*/", Some("/* a */")),
            ("/*  a  */", Some("/* a */")),
            ("/* a */", None),
            ("/* a\t*/", Some("/* a */")),
            ("/*a\nb*/", None),
        ] {
            assert_eq!(
                normalize_comment_sigil_spacing(comment).as_deref(),
                expected,
                "{comment:?}"
            );
        }
    }
}
