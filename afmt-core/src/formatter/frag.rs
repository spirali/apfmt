//! Fragments: alternatives of a doc together with their edges, and the spacing between edges

use crate::comments::trivia_from;
use crate::doc::{DocId, EMPTY_DOC, NEWLINE_DOC, SPACE_DOC};
use crate::formatter::Edge::Nothing;
use ra_ap_syntax::{SyntaxKind, SyntaxToken};
use smallvec::{SmallVec, smallvec};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Edge {
    Nothing,
    Word,
    Keyword,    // let — always followed by a space
    Open,       // ( [ {
    Close,      // ) ] }
    BraceOpen,  // { of a block or struct literal
    BraceClose, // } of a block or struct literal
    Comma,      // , ;
    Tight,      // . :: ! & * # — no space around
    Operator,   // + == => : ->
    Comment,    // start of a comment
    Break,
    /// Start of a fragment that begins with a mandatory line break (`Formatter::on_next_line`): no
    /// space in front of it, and a line comment in front of it needs no line break of its own
    LineStart,
}

/// One alternative of a fragment together with its edges
#[derive(Clone, Copy, Debug)]
pub(super) struct Branch {
    pub(super) doc_id: DocId,
    pub(super) start: Edge,
    pub(super) end: Edge,
}

pub(super) type Branches = SmallVec<[Branch; 1]>;

#[derive(Clone, Debug)]
pub struct Frag {
    pub(super) branches: Branches, // never empty
    /// The last source token of the fragment; the trivia after it (comments, blank lines) is not
    /// placed yet. `None` when the fragment does not cover the source after its tokens
    /// (see `Formatter::token_without_cursor`).
    pub(super) cursor: Option<SyntaxToken>,
}

impl Frag {
    pub(super) fn new(doc_id: DocId, start: Edge, end: Edge, cursor: Option<SyntaxToken>) -> Self {
        Frag {
            branches: smallvec![Branch { start, doc_id, end }],
            cursor,
        }
    }

    pub(super) fn new_simple(doc_id: DocId, edge: Edge, cursor: Option<SyntaxToken>) -> Self {
        Frag::new(doc_id, edge, edge, cursor)
    }

    pub fn empty() -> Self {
        Frag::new(EMPTY_DOC, Nothing, Nothing, None)
    }

    /// The fragment with `cursor` as its last source token instead of its own, for a fragment built
    /// from a copy of the source text that ends where `cursor` does (e.g. a macro's arguments parsed
    /// again): the trivia after `cursor` is placed after the fragment
    pub fn with_cursor(mut self, cursor: SyntaxToken) -> Self {
        self.cursor = Some(cursor);
        self
    }

    /// Whether a layout of the fragment may end with a closing delimiter, e.g. a call `f(...)`
    pub fn may_end_with_delimiter(&self) -> bool {
        self.branches
            .iter()
            .any(|b| matches!(b.end, Edge::Close | Edge::BraceClose))
    }

    /// Whether a comment follows the cursor in the source, before the next code token
    pub fn comment_follows(&self) -> bool {
        let next = self.cursor.as_ref().and_then(|c| c.next_token());
        trivia_from(next).any(|t| t.kind() == SyntaxKind::COMMENT)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.branches.len() == 1 && self.branches[0].doc_id.is_empty()
    }
}

impl Default for Frag {
    fn default() -> Self {
        Frag::empty()
    }
}

pub(super) fn compute_space(left: Edge, right: Edge) -> DocId {
    use Edge::*;
    match (left, right) {
        (_, LineStart) => EMPTY_DOC,
        (Break, _) | (_, Break) => NEWLINE_DOC,
        (Nothing, _) | (_, Nothing) => EMPTY_DOC,
        (BraceOpen, BraceClose) => EMPTY_DOC,          // {}
        (_, Comma) => EMPTY_DOC,                       // x,
        (Comma, Close) => EMPTY_DOC,                   // (x,)
        (Comma, _) => SPACE_DOC,                       // , x
        (BraceOpen, _) | (_, BraceClose) => SPACE_DOC, // { x }
        (Open, _) | (_, Close) => EMPTY_DOC,           // (x)
        (Keyword, _) => SPACE_DOC,                     // let (a, b)  let &x
        (Operator, Tight) => SPACE_DOC,                // a == !b
        (Tight, Comment) => SPACE_DOC,                 // a:: /* c */ b
        (Tight, _) | (_, Tight) => EMPTY_DOC,          // a.b  a::b  &x
        (_, BraceOpen) => SPACE_DOC,                   // f() {
        (Operator, _) | (_, Operator) => SPACE_DOC,    // a + b
        (Word, Open) => EMPTY_DOC,                     // foo(
        (Close | BraceClose, Open) => EMPTY_DOC,       // )(  { .. }(
        (Word, Word) => SPACE_DOC,                     // fn foo
        (Close, Word) => SPACE_DOC,
        _ => SPACE_DOC,
    }
}
