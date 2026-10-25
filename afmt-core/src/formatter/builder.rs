use super::Formatter;
use super::frag::{Branch, Branches, Edge, Frag, compute_space};
use crate::comments::TriviaStop;
use crate::comments::own_line_comment_follows;
use crate::doc::{AlignGroup, DocId, EMPTY_DOC, EMPTYLINE_DOC, NEWLINE_DOC, NEWLINE_OR_SPACE_DOC};
use crate::formatter::Edge::{BraceClose, BraceOpen, Break, Close, LineStart, Nothing};
use ra_ap_syntax::SyntaxToken;

/// Start edges of fragments that continue the construct on their left: everything except closing
/// delimiters and the `{` of a block. Own-line comments in front of them are indented together with them.
fn is_continuation(edge: Edge) -> bool {
    !matches!(edge, Close | BraceClose | BraceOpen)
}

#[derive(Clone)]
pub(crate) struct Builder<'a> {
    pub(super) fmt: &'a Formatter<'a>,
    pub(super) frag: Frag,
    pub(super) align_group: Option<AlignGroup>,
}

impl<'a> Builder<'a> {
    pub fn new(fmt: &'a Formatter) -> Self {
        Builder {
            fmt,
            frag: Frag::empty(),
            align_group: None,
        }
    }

    pub fn with_cursor(fmt: &'a Formatter, cursor: Option<SyntaxToken>) -> Self {
        let mut frag = Frag::empty();
        frag.cursor = cursor;
        Builder {
            fmt,
            frag,
            align_group: None,
        }
    }

    pub fn align_trailing_comments(&mut self, group: Option<AlignGroup>) {
        self.align_group = group;
    }

    /// The fragments one after another, see `push`
    pub fn concat(fmt: &'a Formatter, frags: impl IntoIterator<Item = Frag>) -> Frag {
        let mut builder = Builder::new(fmt);
        for frag in frags {
            builder.push(frag);
        }
        builder.into_frag()
    }

    #[inline]
    pub fn into_frag(self) -> Frag {
        self.frag
    }

    pub fn is_empty(&self) -> bool {
        self.frag.is_empty()
    }

    /// The fragment built so far, leaving the builder empty, if it ends at the start of a line with
    /// no trivia pending (every branch ends with a line break, no cursor); what follows cannot
    /// change its layout
    pub fn split_off_finished_lines(&mut self) -> Option<Frag> {
        let finished = !self.frag.is_empty()
            && self.frag.cursor.is_none()
            && self.frag.branches.iter().all(|b| b.end == Nothing);
        finished.then(|| std::mem::take(&mut self.frag))
    }

    /// Appends line-breaking `doc_id` to the branches whose end edge satisfies `pred`; they then end with nothing
    pub(super) fn push_line_break_if(&mut self, doc_id: DocId, pred: impl Fn(Edge) -> bool) {
        let fmt = self.fmt;
        let frag = std::mem::take(&mut self.frag);
        self.frag = fmt.map_branches(frag, |b| {
            if pred(b.end) {
                Branch {
                    doc_id: fmt.concat(b.doc_id, doc_id),
                    end: Nothing,
                    ..b
                }
            } else {
                b
            }
        });
    }

    pub fn cursor(&self) -> Option<&SyntaxToken> {
        self.frag.cursor.as_ref()
    }

    pub fn push_newline(&mut self) {
        self.push_line_break_if(NEWLINE_DOC, |_| true);
    }

    /// Ends the line and leaves a blank line after it
    pub fn push_blank_line(&mut self) {
        self.push_line_break_if(EMPTYLINE_DOC, |_| true);
    }

    /// Pushes `frag` on a new line; the first item of the builder stays where it is.
    /// Blank lines from the source in front of it are kept.
    pub fn push_on_new_line(&mut self, frag: Frag) {
        if !self.is_empty() {
            self.flush_trivia_and_end_line();
        }
        self.push(frag);
    }

    /// A space or a line break, whichever gives the better layout
    pub fn push_soft_line_break(&mut self) {
        self.push_line_break_if(NEWLINE_OR_SPACE_DOC, |_| true);
    }

    pub fn push_token(&mut self, token: SyntaxToken, edge: Edge) {
        self.push(self.fmt.token(token, edge));
    }

    pub fn push_keyword(&mut self, token: SyntaxToken) {
        self.push(self.fmt.keyword(token));
    }

    pub fn push(&mut self, frag: Frag) {
        if frag.is_empty() {
            return;
        }
        if self.frag.is_empty() {
            self.frag = frag;
            return;
        }
        if self.frag.cursor.is_some() && frag.branches.iter().all(|b| is_continuation(b.start)) {
            self.push_continuation(frag);
            return;
        }
        self.flush_trivia();
        self.join(frag);
    }

    /// Pushes each of `lines` on its own line, all of them indented one level deeper with `indent`,
    /// e.g. the continuation lines of a broken chain (`.b()`, `&& b`). Comments on the line of the left
    /// side stay there; own-line comments in between keep their own lines, at the indentation of
    /// the lines.
    pub fn push_lines(&mut self, lines: impl IntoIterator<Item = Frag>, indent: bool) {
        let mut lines = lines.into_iter().peekable();
        if lines.peek().is_none() {
            return;
        }
        let fmt = self.fmt;
        self.push_trivia(None, TriviaStop::OwnLine);
        let mut nested = Builder::with_cursor(fmt, self.frag.cursor.take());
        // The newline in front of the first line belongs to `nested`, so it is indented with it
        nested.push_newline();
        for line in lines {
            nested.push_on_new_line(line);
        }
        let nested = nested.into_frag();
        // A line comment on the left side is ended by the newline of `nested`
        self.push_line_break_if(EMPTY_DOC, |end| end == Break);
        self.join(if indent { fmt.nest(nested) } else { nested });
    }

    /// Takes the source up to `token` (inclusive) without output, e.g. the `}` of braces removed around a
    /// closure body; the source in between must not contain comments
    pub fn skip_to_token(&mut self, token: SyntaxToken) {
        self.push_comments();
        self.frag.cursor = Some(token);
    }

    /// `frag` followed by the source up to `token` (inclusive) taken without output, see
    /// `skip_to_token`
    pub fn skip_to_token_after(fmt: &'a Formatter, frag: Frag, token: SyntaxToken) -> Frag {
        let mut builder = Builder::new(fmt);
        builder.push(frag);
        builder.skip_to_token(token);
        builder.into_frag()
    }

    /// Pushes `doc`, text that is not in the source, e.g. braces added around a closure body
    /// (`OPEN_BRACE_DOC`, ...: generated text is preallocated). The cursor stays, so the trivia after
    /// it is placed after the text, and the source that follows is still taken after it (e.g. a
    /// separator).
    pub fn push_generated(&mut self, doc: DocId, edge: Edge) {
        let cursor = self.frag.cursor.take();
        let frag = Frag::new_simple(doc, edge, None);
        if self.frag.is_empty() {
            self.frag = frag;
        } else {
            self.join(frag);
        }
        self.frag.cursor = cursor;
    }

    fn push_continuation(&mut self, frag: Frag) {
        // Comments on the same line as the left side stay there
        self.push_trivia(None, TriviaStop::OwnLine);
        // After an own-line comment, or a line comment ending the left side, the continuation goes on
        // an indented line, unless it starts its own line anyway
        let own_line_comment = self
            .frag
            .cursor
            .as_ref()
            .is_some_and(own_line_comment_follows);
        let line_comment = self.frag.branches.iter().any(|b| b.end == Break)
            && !frag.branches.iter().all(|b| b.start == LineStart);
        if !own_line_comment && !line_comment {
            self.flush_trivia();
            self.join(frag);
            return;
        }
        let fmt = self.fmt;
        let mut nested = Builder::with_cursor(fmt, self.frag.cursor.take());
        nested.flush_trivia();
        nested.push(frag);
        // A line comment on the left side is ended by the nested newline
        self.push_line_break_if(EMPTY_DOC, |end| end == Break);
        self.join(fmt.indented_line(nested.into_frag()));
    }

    /// Concatenates `frag` without processing the trivia in between
    fn join(&mut self, frag: Frag) {
        let fmt = self.fmt;
        let left = &self.frag.branches;
        let mut branches = Branches::new();
        for right in &frag.branches {
            for (i, first) in left.iter().enumerate() {
                if left[..i].iter().any(|l| l.start == first.start) {
                    continue;
                }
                let doc_id = left[i..]
                    .iter()
                    .filter(|l| l.start == first.start)
                    .map(|l| fmt.concat(l.doc_id, compute_space(l.end, right.start)))
                    .reduce(|a, b| fmt.alt_doc(a, b))
                    .unwrap();
                branches.push(Branch {
                    doc_id: fmt.concat(doc_id, right.doc_id),
                    start: first.start,
                    end: right.end,
                });
            }
        }
        self.frag.branches = fmt.group_branches(branches);
        self.frag.cursor = frag.cursor
    }
}
