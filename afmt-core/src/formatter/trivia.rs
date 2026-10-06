use super::Builder;
use super::frag::{Edge, Frag};
use crate::doc::{COMMA_DOC, DocId, EMPTY_DOC, EMPTYLINE_DOC, NEWLINE_DOC, SEMICOLON_DOC};
use crate::formatter::Edge::{Break, Comma, Nothing, Word};
use ra_ap_syntax::{SyntaxKind, SyntaxToken};

fn count_newlines(token: &SyntaxToken) -> u32 {
    token.text().bytes().filter(|c| *c == b'\n').count() as u32
}

/// An own-line block comment directly followed by code on the same line, e.g. `/* c */ f();`
fn is_leading_block_comment(token: &SyntaxToken) -> bool {
    let has_newline = |t: Option<SyntaxToken>| {
        t.is_some_and(|t| t.kind() == SyntaxKind::WHITESPACE && count_newlines(&t) > 0)
    };
    // The start of the source is the start of a line too
    let line_start = token.prev_token().is_none() || has_newline(token.prev_token());
    token.kind() == SyntaxKind::COMMENT
        && token.text().starts_with("/*")
        && line_start
        && !has_newline(token.next_token())
}

/// Where `Builder::scan_comments` stops at the latest
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ScanStop {
    /// In front of the first token that is not trivia
    Code,
    /// In front of the first own-line comment
    OwnLine,
    /// Right after the delimiter; comments behind it are left
    Delimiter,
}

fn delimiter_doc(kind: SyntaxKind) -> DocId {
    match kind {
        SyntaxKind::SEMICOLON => SEMICOLON_DOC,
        SyntaxKind::COMMA => COMMA_DOC,
        _ => unreachable!(),
    }
}

/// Whether the rest of the line after `token` holds only comments and the delimiter `kind` (if given),
/// i.e. no code follows on the same line after the delimiter
fn rest_of_line_is_comments(token: &SyntaxToken, delimiter: Option<SyntaxKind>) -> bool {
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

impl<'a> Builder<'a> {
    /// Ends the line after a trailing line comment, which would otherwise swallow the following code
    pub fn end_line_after_line_comment(&mut self) {
        self.push_line_break_if(NEWLINE_DOC, |end| end == Break);
    }

    /// Ends the line; blank lines from the source are kept
    pub fn ensure_newline(&mut self) {
        self.consume_comments();
        // The comment belongs to the following code, so no line break after it
        let leading_comment = self
            .frag
            .cursor
            .as_ref()
            .is_some_and(is_leading_block_comment);
        self.flush_trivia(true);
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

    fn flush_empty_lines(&mut self, newlines: u32) {
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

    /// Consumes the comments after the cursor (`frag.cursor`) and moves the cursor to the last consumed token.
    /// The whitespace after the cursor is left for `flush_trivia`.
    pub fn consume_comments(&mut self) {
        if let Some(cursor) = self.frag.cursor.take() {
            self.frag.cursor = Some(self.scan_comments(cursor, None, ScanStop::Code));
        }
    }

    /// Pushes the comments following `token`; the source `delimiter`, if found among them, is consumed
    /// without printing. `stop` says where the scan ends at the latest.
    /// Returns the last consumed token.
    /// Pushes the comment `token`; a line comment ends the line (`Break`)
    fn push_comment(&mut self, token: &SyntaxToken) {
        let end = if token.text().starts_with("//") {
            Break
        } else {
            Word
        };
        let doc_id = self.fmt.comment_doc(token);
        self.push(Frag::new(doc_id, Edge::Comment, end, None));
    }

    /// Pushes the comments starting at `token` (inclusive) up to the first code token, for trivia that no
    /// cursor precedes, e.g. at the start of the source; after the content already in the builder, they
    /// start on a new line. The last comment becomes the cursor.
    pub fn push_comments_from(&mut self, token: Option<SyntaxToken>) {
        let first_comment = std::iter::successors(token, |t| t.next_token())
            .take_while(|t| t.kind().is_trivia())
            .find(|t| t.kind() == SyntaxKind::COMMENT);
        if let Some(comment) = first_comment {
            self.push_line_break_if(NEWLINE_DOC, |end| end != Nothing);
            self.push_comment(&comment);
            self.frag.cursor = Some(comment);
            self.consume_comments();
        }
    }

    pub(super) fn scan_comments(
        &mut self,
        token: SyntaxToken,
        mut delimiter: Option<SyntaxKind>,
        stop: ScanStop,
    ) -> SyntaxToken {
        let mut cursor = token;
        let mut newlines = 0;
        let mut element = cursor.next_token();
        if delimiter.is_some() {
            // The delimiter is only taken from the source when it follows the comments
            let next = std::iter::successors(element.clone(), |t| t.next_token())
                .find(|t| !matches!(t.kind(), SyntaxKind::WHITESPACE | SyntaxKind::COMMENT));
            if next.map(|t| t.kind()) != delimiter {
                delimiter = None;
            }
        }
        while let Some(token) = element {
            match token.kind() {
                SyntaxKind::WHITESPACE => {
                    newlines += count_newlines(&token);
                }
                SyntaxKind::COMMENT => {
                    if delimiter.is_some() {
                        // The comment is moved after the delimiter, so it stays on the delimiter's line
                        newlines = 0;
                    }
                    let own_line = newlines > 0;
                    if own_line && stop != ScanStop::Code {
                        break;
                    }
                    // Blank lines at the start of a fragment are dropped; the parent decides spacing
                    if newlines > 0 && !self.frag.is_empty() {
                        self.flush_empty_lines(newlines);
                    }
                    newlines = 0;
                    if own_line {
                        self.push_line_break_if(NEWLINE_DOC, |end| end != Nothing);
                    }
                    self.push_comment(&token);
                    cursor = token.clone();
                }
                x if Some(x) == delimiter => {
                    // The delimiter is moved before the comments, so newlines in front of it do not count
                    newlines = 0;
                    delimiter = None;
                    cursor = token.clone();
                    if stop == ScanStop::Delimiter {
                        break;
                    }
                }
                _ => break,
            }
            element = token.next_token();
        }
        cursor
    }

    /// Breaks the line after the cursor: the comments on the cursor's line stay there, then a line break,
    /// then the other comments (own-line ones). A line break after them is kept when the source has one.
    /// E.g. after the opening delimiter of a vertical list.
    pub fn break_after_line_comments(&mut self) {
        if let Some(cursor) = self.frag.cursor.take() {
            self.frag.cursor = Some(self.scan_comments(cursor, None, ScanStop::OwnLine));
        }
        self.push_newline();
        self.consume_comments();
        let Some(cursor) = self.frag.cursor.take() else {
            return;
        };
        if cursor
            .next_token()
            .is_some_and(|t| t.kind() == SyntaxKind::WHITESPACE && count_newlines(&t) > 0)
        {
            self.push_line_break_if(NEWLINE_DOC, |end| end != Nothing);
        }
    }

    /// Consumes the trivia after the cursor and clears it; blank lines at the end are kept
    /// only with `respect_newlines`
    pub fn flush_trivia(&mut self, respect_newlines: bool) {
        self.consume_comments();
        let Some(cursor) = self.frag.cursor.take() else {
            return;
        };
        if respect_newlines {
            let mut newlines = 0;
            let mut element = cursor.next_token();
            while let Some(token) = element.filter(|t| t.kind() == SyntaxKind::WHITESPACE) {
                newlines += count_newlines(&token);
                element = token.next_token();
            }
            if newlines > 1 {
                self.flush_empty_lines(newlines);
            }
        }
    }

    pub fn push_delimiter(&mut self, kind: SyntaxKind) {
        self.consume_delimiter(kind, true);
    }

    /// Like `push_delimiter`, but the delimiter is not printed (e.g. an optional trailing comma)
    pub fn skip_delimiter(&mut self, kind: SyntaxKind) {
        self.consume_delimiter(kind, false);
    }

    /// Consumes the source delimiter `kind` after the cursor (if there is one) together with
    /// the comments around it; the comments are placed after the delimiter
    pub fn consume_delimiter(&mut self, kind: SyntaxKind, emit: bool) {
        let cursor = self.frag.cursor.take();
        if emit {
            self.push(Frag::new_simple(delimiter_doc(kind), Comma, None));
        }
        if let Some(cursor) = cursor {
            self.frag.cursor = Some(self.scan_comments(cursor, Some(kind), ScanStop::Code));
        }
    }

    /// Consumes the delimiter `kind` after a list item (the cursor) together with the item's post
    /// comments, as rustfmt calls them: the comments in front of the delimiter and the comments after
    /// it that end the line. Comments followed by code on the same line and own-line comments are
    /// pre comments of the next item, so they are left for `push_leading_comments`.
    /// Returns whether a post comment was pushed.
    pub fn push_separator_with_trailing_comments(&mut self, kind: SyntaxKind) -> bool {
        let token = self.frag.cursor.take();
        self.push(Frag::new_simple(delimiter_doc(kind), Comma, None));
        let Some(token) = token else {
            return false;
        };
        let stop = if rest_of_line_is_comments(&token, Some(kind)) {
            ScanStop::OwnLine
        } else {
            ScanStop::Delimiter
        };
        let cursor = self.scan_comments(token.clone(), Some(kind), stop);
        let had_comment = cursor != token
            && std::iter::successors(token.next_token(), |t| t.next_token())
                .take_while(|t| t.text_range().start() <= cursor.text_range().start())
                .any(|t| t.kind() == SyntaxKind::COMMENT);
        self.frag.cursor = Some(cursor);
        had_comment
    }

    /// Pushes the comments in front of the next list item (rustfmt's pre comments). A line break
    /// follows them when the source has one after them, otherwise the item follows after a space.
    pub fn push_leading_comments(&mut self) {
        self.consume_comments();
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

    /// Pushes the comments after the cursor if nothing but comments follows on its line, e.g.
    /// `( /* c */`; they stay on the cursor's line. A line comment no longer ends the line itself,
    /// the caller has to start a new line.
    pub fn push_comments_ending_line(&mut self) {
        let Some(cursor) = self.frag.cursor.take() else {
            return;
        };
        if rest_of_line_is_comments(&cursor, None) {
            self.frag.cursor = Some(self.scan_comments(cursor, None, ScanStop::OwnLine));
            self.push_line_break_if(EMPTY_DOC, |end| end == Break);
        } else {
            self.frag.cursor = Some(cursor);
        }
    }

    pub fn has_own_line_comment(&mut self) -> bool {
        let Some(cursor) = self.frag.cursor.as_ref() else {
            return false;
        };
        let mut token = cursor.next_token();
        let mut line_count = 0;
        while let Some(t) = token {
            match t.kind() {
                SyntaxKind::WHITESPACE => {
                    line_count += count_newlines(&t);
                }
                SyntaxKind::COMMENT if line_count > 0 => return true,
                SyntaxKind::COMMENT => {}
                _ => break,
            }
            token = t.next_token();
        }
        false
    }
}
