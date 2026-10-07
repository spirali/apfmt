use super::Formatter;
use crate::comments::own_line_comment_follows;
use super::frag::{Branch, Branches, Edge, Frag, compute_space};
use crate::comments::TriviaStop;
use crate::doc::{DocId, EMPTY_DOC, NEWLINE_DOC, NEWLINE_OR_SPACE_DOC};
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
}

impl<'a> Builder<'a> {
    pub fn new(fmt: &'a Formatter) -> Self {
        Builder {
            fmt,
            frag: Frag::empty(),
        }
    }

    pub fn with_cursor(fmt: &'a Formatter, cursor: Option<SyntaxToken>) -> Self {
        let mut frag = Frag::empty();
        frag.cursor = cursor;
        Builder { fmt, frag }
    }

    #[inline]
    pub fn into_frag(self) -> Frag {
        self.frag
    }

    pub fn is_empty(&self) -> bool {
        self.frag.is_empty()
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

    fn push_continuation(&mut self, frag: Frag) {
        // Comments on the same line as the left side stay there
        self.push_trivia(None, TriviaStop::OwnLine);
        // After an own-line comment, or a line comment ending the left side, the continuation goes on
        // an indented line (taxonomy: ContinuationComment), unless it starts its own line anyway
        let own_line_comment = self.frag.cursor.as_ref().is_some_and(own_line_comment_follows);
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
