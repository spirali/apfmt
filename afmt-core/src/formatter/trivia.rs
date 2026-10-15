use super::Builder;
use super::frag::{Edge, Frag};
use crate::comments::{
    TriviaStop, count_newlines, is_leading_block_comment, is_line_comment, line_ends_after,
    next_code_token, post_comments_stop, rest_of_line_is_comments, trivia_end, trivia_from,
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

/// Puts a single space after the opening sigil of a line comment (`//a` -> `// a`) and after the
/// opening and before the closing sigil of a single-line block comment (`/*a*/` -> `/* a */`), as the
/// Style Guide recommends, when the comment text starts with a letter or a digit; other comments
/// (empty ones, separators `//-----`, commented-out code `//#[cfg(x)]`, ...) are left alone, and so
/// are doc comments (adding a space to some lines of a doc comment would change the indentation of
/// its Markdown), `////...` and multi-line block comments. Returns `None` when the text stays as it is.
pub(super) fn normalize_comment_sigil_spacing(text: &str) -> Option<String> {
    let starts_with_alphanumeric = |s: &str| s.starts_with(char::is_alphanumeric);
    if let Some(rest) = text.strip_prefix("//") {
        let skip = rest.starts_with(['/', '!']) || !starts_with_alphanumeric(rest);
        return (!skip).then(|| format!("// {rest}"));
    }
    let inner = text.strip_prefix("/*")?.strip_suffix("*/")?;
    if inner.contains('\n') || inner.starts_with('*') || inner.starts_with('!') {
        return None;
    }
    let trimmed = inner.trim();
    if !starts_with_alphanumeric(trimmed) {
        return None;
    }
    let normalized = format!("/* {trimmed} */");
    (normalized != text).then_some(normalized)
}

/// A line of a comment without its trailing whitespace, which the Style Guide forbids; a line of a
/// doc comment (`doc`) keeps it, since two trailing spaces end a line in Markdown
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

/// Lines of a multi-line block comment with the indentation of the comment start removed, so they can
/// be aligned to the new column of the comment; blank lines are empty. `None` when the comment is
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
    /// Ends the line after a trailing line comment, which would otherwise swallow the following code
    pub fn end_line_after_line_comment(&mut self) {
        self.push_line_break_if(NEWLINE_DOC, |end| end == Break);
    }

    /// Pushes the trivia after the cursor (comments, and the blank lines of the source, see
    /// `flush_trivia_keeping_blank_lines`) and ends the line, so the next fragment starts a new line;
    /// no line break follows a block comment that belongs to the next code (`/* c */ f();`)
    pub fn flush_trivia_and_end_line(&mut self) {
        self.push_comments();
        // The comment belongs to the following code, so no line break after it
        let leading_comment = self
            .frag
            .cursor
            .as_ref()
            .is_some_and(is_leading_block_comment);
        self.flush_trivia_keeping_blank_lines();
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

    /// Pushes the blank lines for `newlines` source newlines (at most `blank_lines_upper_bound`); a
    /// single newline only ends a line comment, otherwise the caller decides the spacing
    fn push_blank_lines(&mut self, newlines: u32) {
        if newlines == 0 {
            return;
        }
        let mut blank = (newlines - 1).min(self.fmt.config.blank_lines_upper_bound);
        if blank == 0 {
            // A single source newline is significant only after a line comment;
            // otherwise the caller decides spacing
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

    /// Pushes the comments after the cursor (`frag.cursor`), up to the next code token, and moves the
    /// cursor to the last of them; the whitespace after it is left for `flush_trivia`
    pub fn push_comments(&mut self) {
        self.push_trivia(None, TriviaStop::Code);
    }

    /// Pushes the comment `token`; a line comment ends the line (`Break`)
    fn push_comment(&mut self, token: &SyntaxToken, trailing: bool) {
        let end = if is_line_comment(token) { Break } else { Word };
        let mut doc_id = self.fmt.comment_doc(token);
        if let Some(group) = self.align_group
            && trailing
            && !token.text().contains('\n')
        {
            doc_id = self.fmt.concat(self.fmt.align_mark(group), doc_id);
        }
        self.push(Frag::new(doc_id, Edge::Comment, end, None));
    }

    /// Pushes the comments starting at `token` (inclusive) up to the first code token, for trivia that no
    /// cursor precedes, e.g. at the start of the source; after the content already in the builder, they
    /// start on a new line. The last comment becomes the cursor.
    pub fn push_comments_from(&mut self, token: Option<SyntaxToken>) {
        let first_comment = trivia_from(token).find(|t| t.kind() == SyntaxKind::COMMENT);
        if let Some(comment) = first_comment {
            self.push_line_break_if(NEWLINE_DOC, |end| end != Nothing);
            self.push_comment(&comment, false);
            self.frag.cursor = Some(comment);
            self.push_comments();
        }
    }

    /// Pushes the trivia after the cursor: its comments, with the line breaks and blank lines between them,
    /// up to `stop` at the latest (see `trivia_end`). The source `separator`, if it follows the comments,
    /// is taken without printing (the caller pushed one in front of them). Moves the cursor to the last
    /// token taken and returns whether a comment was pushed.
    pub(super) fn push_trivia(&mut self, separator: Option<SyntaxKind>, stop: TriviaStop) -> bool {
        let Some(cursor) = self.frag.cursor.take() else {
            return false;
        };
        let end = trivia_end(&cursor, separator, stop);
        let mut pushed = false;
        let mut newlines = 0;
        // Comments in front of the separator are moved behind it, so they are never own-line
        let mut before_separator = separator
            .is_some_and(|kind| next_code_token(&cursor).is_some_and(|t| t.kind() == kind));
        let mut element = cursor.next_token();
        while cursor != end
            && let Some(token) = element
        {
            match token.kind() {
                SyntaxKind::WHITESPACE => newlines += count_newlines(&token),
                SyntaxKind::COMMENT => {
                    if before_separator {
                        newlines = 0;
                    }
                    // Blank lines at the start of a fragment are dropped; the parent decides spacing
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
                // The separator is moved before the comments, so newlines in front of it do not count
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

    /// Pushes the comments after the cursor, then clears it, so the trivia after it is done; blank lines
    /// after the last comment are dropped (see `flush_trivia_keeping_blank_lines`)
    pub fn flush_trivia(&mut self) {
        self.push_comments();
        self.frag.cursor = None;
    }

    /// Like `flush_trivia`, but the blank lines after the last comment are kept (up to
    /// `blank_lines_upper_bound`), e.g. between statements
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

    /// Pushes the separator `kind` (`,` or `;`, a generated one) and takes the source separator after the
    /// cursor, if there is one, together with the comments around it; the comments are placed after it
    pub fn push_separator(&mut self, kind: SyntaxKind) {
        self.take_separator(kind, true);
    }

    /// Like `push_separator`, but the separator is not printed (e.g. an optional trailing comma)
    pub fn skip_separator(&mut self, kind: SyntaxKind) {
        self.take_separator(kind, false);
    }

    /// Takes the source separator `kind` after the cursor (if there is one) together with the comments
    /// around it, which are pushed after it; with `push`, a separator is pushed in front of them
    fn take_separator(&mut self, kind: SyntaxKind, push: bool) {
        // Pushed without processing the trivia after the cursor, which follows the separator
        let cursor = self.frag.cursor.take();
        if push {
            self.push(Frag::new_simple(separator_doc(kind), Comma, None));
        }
        self.frag.cursor = cursor;
        self.push_trivia(Some(kind), TriviaStop::Code);
    }

    /// Pushes a separator `kind` after a list item (the cursor) and takes the source one, together with
    /// the item's post comments, as rustfmt calls them: the comments in front of the separator and the
    /// comments after it that end the line. Comments followed by code on the same line and own-line comments are
    /// pre comments of the next item, so they are left for `push_leading_comments`.
    /// Returns whether a post comment was pushed.
    pub fn push_separator_with_trailing_comments(&mut self, kind: SyntaxKind) -> bool {
        // Pushed without processing the trivia after the cursor, which follows the separator
        let cursor = self.frag.cursor.take();
        self.push(Frag::new_simple(separator_doc(kind), Comma, None));
        let Some(cursor) = cursor else {
            return false;
        };
        let stop = post_comments_stop(&cursor, kind);
        self.frag.cursor = Some(cursor);
        self.push_trivia(Some(kind), stop)
    }

    /// Pushes the comments in front of the next list item (rustfmt's pre comments). A line break
    /// follows them when the source has one after them, otherwise the item follows after a space.
    pub fn push_leading_comments(&mut self) {
        self.push_comments();
        let Some(cursor) = self.frag.cursor.as_ref() else {
            return;
        };
        if cursor.kind() == SyntaxKind::COMMENT
            && cursor
                .next_token()
                .is_some_and(|t| t.kind() == SyntaxKind::WHITESPACE && count_newlines(&t) > 0)
        {
            self.push_line_break_if(NEWLINE_DOC, |end| end != Nothing);
        }
    }

    /// Pushes the comments after the cursor if nothing but comments follows on its line, e.g. `( // c`
    /// or `{ /* c */`; a comment followed by code on the line is left for that code. Returns whether it
    /// pushed any; the code after them starts on a new line. Used at the start of a block or a broken
    /// list, placed with `Formatter::body_after_opener`.
    pub fn push_comments_ending_line(&mut self) -> bool {
        let ends_line = self
            .frag
            .cursor
            .as_ref()
            .is_some_and(|c| rest_of_line_is_comments(c, None));
        ends_line && self.push_trivia(None, TriviaStop::OwnLine)
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
