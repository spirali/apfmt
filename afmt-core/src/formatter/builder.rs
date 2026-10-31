use super::Formatter;
use super::frag::{Branch, Branches, Edge, Frag, compute_space};
use crate::comments::TriviaStop;
use crate::comments::own_line_comment_follows;
use crate::doc::{AlignGroup, DocId, EMPTY_DOC, EMPTYLINE_DOC, NEWLINE_DOC, NEWLINE_OR_SPACE_DOC};
use crate::formatter::Edge::{BraceClose, BraceOpen, Break, Close, LineStart, Nothing};
use ra_ap_syntax::SyntaxToken;

/// Everything except closing delimiters and the `{` of a block; own-line comments in front of them
/// are indented with them
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

    pub fn concat(fmt: &'a Formatter, frags: impl IntoIterator<Item = Frag>) -> Frag {
        let mut builder = Builder::new(fmt);
        for frag in frags {
            builder.push(frag);
        }
        builder.into_frag()
    }

    /// `head` followed by `lines`, each on a line of its own, block-indented with `indent`
    pub fn head_then_lines(
        fmt: &'a Formatter,
        head: Frag,
        lines: impl IntoIterator<Item = Frag>,
        indent: bool,
    ) -> Frag {
        let mut builder = Builder::new(fmt);
        builder.push(head);
        builder.push_lines(lines, indent);
        builder.into_frag()
    }

    #[inline]
    pub fn into_frag(self) -> Frag {
        self.frag
    }

    pub fn is_empty(&self) -> bool {
        self.frag.is_empty()
    }

    /// The fragment built so far, if every branch ends with a line break and no trivia is pending:
    /// what follows cannot change its layout
    pub fn split_off_finished_lines(&mut self) -> Option<Frag> {
        let finished = !self.frag.is_empty()
            && self.frag.cursor.is_none()
            && self.frag.branches.iter().all(|b| b.end == Nothing);
        finished.then(|| std::mem::take(&mut self.frag))
    }

    /// They then end with nothing
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

    pub fn push_blank_line(&mut self) {
        self.push_line_break_if(EMPTYLINE_DOC, |_| true);
    }

    /// The first item of the builder stays where it is; blank lines from the source are kept
    pub fn push_on_new_line(&mut self, frag: Frag) {
        if !self.is_empty() {
            self.flush_trivia_and_end_line();
        }
        self.push(frag);
    }

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

    /// Each of `lines` on its own line, indented with `indent`, e.g. `.b()`, `&& b`; own-line
    /// comments in between keep their own lines
    pub fn push_lines(&mut self, lines: impl IntoIterator<Item = Frag>, indent: bool) {
        let mut lines = lines.into_iter().peekable();
        if lines.peek().is_none() {
            return;
        }
        let fmt = self.fmt;
        self.push_trivia(None, TriviaStop::OwnLine);
        let mut nested = Builder::with_cursor(fmt, self.frag.cursor.take());
        // The newline belongs to `nested`, so it is indented with it
        nested.push_newline();
        for line in lines {
            nested.push_on_new_line(line);
        }
        let nested = nested.into_frag();
        self.push_line_break_if(EMPTY_DOC, |end| end == Break);
        self.join(if indent { fmt.nest(nested) } else { nested });
    }

    /// E.g. the `}` of braces removed around a closure body; the source in between must not contain
    /// comments
    pub fn skip_to_token(&mut self, token: SyntaxToken) {
        self.push_comments();
        self.frag.cursor = Some(token);
    }

    pub fn skip_to_token_after(fmt: &'a Formatter, frag: Frag, token: SyntaxToken) -> Frag {
        let mut builder = Builder::new(fmt);
        builder.push(frag);
        builder.skip_to_token(token);
        builder.into_frag()
    }

    /// Text that is not in the source (preallocated, e.g. `OPEN_BRACE_DOC`); the cursor stays, so
    /// the trivia and source after it are taken after the text
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
        // After an own-line comment or a line comment, the continuation goes on an indented line
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
        self.push_line_break_if(EMPTY_DOC, |end| end == Break);
        self.join(fmt.indented_line(nested.into_frag()));
    }

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
