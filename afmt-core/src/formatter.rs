
use std::cell::RefCell;
use crate::config::Config;
use crate::doc::DocNode::Newline;
use crate::doc::{DocArena, DocId, DocNode, EMPTY_DOC, EMPTYLINE_DOC, NEWLINE_DOC, NEWLINE_OR_SPACE_DOC, SPACE_DOC, SEMICOLON_DOC, COMMA_DOC};
use crate::resolver::Resolver;
use ra_ap_syntax::{ast, PreorderWithTokens, RustLanguage, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken, WalkEvent, AstNode, SourceFile, Direction};

use ra_ap_syntax::ast::{ArgList, BlockExpr, CallExpr, Comment, Expr, HasArgList, HasModuleItem, HasName, Item, Literal, Name, Path, PathExpr, PathSegment, Stmt, StmtList, Fn as AstFn, ParamList};
use crate::formatter::Edge::{Break, BraceClose, BraceOpen, Close, Comma, Nothing, Open, Operator, Tight, Word};
use smallvec::{SmallVec, smallvec};

pub(crate) struct Formatter<'a> {
    doc_arena: RefCell<DocArena>,
    config: &'a Config,
}

impl<'a> Formatter<'a> {
    pub fn new(config: &'a Config) -> Self {
        Formatter {
            config,
            doc_arena: RefCell::new(DocArena::new()),
        }
    }

    #[inline]
    pub fn config(&self) -> &'a Config {
        self.config
    }

    pub fn make_node(&self, doc_node: DocNode) -> DocId {
        self.doc_arena.borrow_mut().make(doc_node)
    }

    pub fn token(&self, token: SyntaxToken, edge: Edge) -> Frag {
        Frag::new_simple(self.make_node(DocNode::Token(token.clone())), edge, Some(token))
    }

    pub fn text(&self, text: String, edge: Edge) -> Frag {
        Frag::new_simple(self.make_node(DocNode::Text(text)), edge, None)
    }

    pub fn concat(&self, doc1: DocId, doc2: DocId) -> DocId {
        if doc1 == EMPTY_DOC {
            return doc2;
        }
        if doc2 == EMPTY_DOC {
            return doc1;
        }
        self.make_node(DocNode::Concat(doc1, doc2))
    }

    pub fn nest(&self, frag: Frag) -> Frag {
        self.map_branches(frag, |b| Branch { doc_id: self.make_node(DocNode::Nest(b.doc_id)), ..b })
    }

    /// Restricts the fragment to layouts on a single line
    pub fn flat(&self, frag: Frag) -> Frag {
        self.map_branches(frag, |b| Branch { doc_id: self.make_node(DocNode::Flat(b.doc_id)), ..b })
    }

    pub fn alt_doc(&self, doc1: DocId, doc2: DocId) -> DocId {
        if doc1 == doc2 {
            return doc1;
        }
        self.make_node(DocNode::Alt(doc1, doc2))
    }

    pub fn alt_frag(&self, frag1: Frag, frag2: Frag) -> Frag {
        // Empty lines come from the source after the node, so they cannot depend on the chosen branch
        debug_assert_eq!(frag1.token, frag2.token);
        let mut branches = frag1.branches;
        branches.extend(frag2.branches);
        Frag {
            branches: self.group_branches(branches),
            token: frag1.token,
        }
    }

    pub fn map_branches(&self, mut frag: Frag, f: impl FnMut(Branch) -> Branch) -> Frag {
        let branches = frag.branches.into_iter().map(f).collect();
        frag.branches = self.group_branches(branches);
        frag
    }

    /// Merges branches with the same edges into a single branch
    fn group_branches(&self, branches: Branches) -> Branches {
       if branches.len() <= 1 {
           return branches;
       }
        let mut out = Branches::new();
        for b in branches {
            if let Some(o) = out.iter_mut().find(|o| o.start == b.start && o.end == b.end) {
                o.doc_id = self.alt_doc(o.doc_id, b.doc_id);
            } else {
                out.push(b);
            }
        }
        out
    }

    /// Doc of the whole fragment; the edges are dropped
    pub fn frag_to_doc(&self, frag: &Frag) -> DocId {
        frag.branches
            .iter()
            .map(|b| b.doc_id)
            .reduce(|a, b| self.alt_doc(a, b))
            .unwrap()
    }

/*
    pub fn comma_separated(&self, docs: &[DocId]) -> DocId {
        let mut it = docs.iter();
        let Some(doc) = it.next() else {
            return EMPTY_DOC;
        };
        let mut doc = *doc;
        for d in it {
            doc = self.concat_s_text(doc, ", ");
            doc = self.concat(doc, *d);
        }
        doc
    }

    pub fn comma_mix_separated(&self, docs: &[DocId]) -> DocId {
        let mut it = docs.iter();
        let Some(doc) = it.next() else {
            return EMPTY_DOC;
        };
        let mut doc = *doc;
        for d in it {
            doc = self.concat_s_text(doc, ",");
            doc = self.concat(doc, NEWLINE_OR_SPACE_DOC);
            doc = self.concat(doc, *d);
        }
        doc = self.nest(doc);
        doc = self.concat(doc, NEWLINE_DOC);
        doc
    }

    pub fn comma_nl_separated(&self, docs: &[DocId]) -> DocId {
        let mut doc = EMPTY_DOC;
        for d in docs.iter() {
            doc = self.concat(doc, *d);
            doc = self.concat_s_text(doc, ",");
            doc = self.concat_new_line(doc)
        }
        doc
    }
*/
    pub fn dump(&self, root: DocId) {
        self.doc_arena.borrow().dump(root);
    }

    pub fn into_resolver(self, root: DocId) -> Resolver<'a> {
        Resolver::new(self.doc_arena.into_inner(), self.config, root)
    }

    pub fn simple_length_doc(&self, doc_id: DocId) -> Option<usize> {
        match self.doc_arena.borrow().get_node(doc_id) {
            DocNode::Token(token) => Some(unicode_width::UnicodeWidthStr::width(token.text())),
            DocNode::Text(t) => Some(unicode_width::UnicodeWidthStr::width(t.as_str())),
            Newline | DocNode::Nest(_) | DocNode::Align(_) => None,
            DocNode::Flat(d) => self.simple_length_doc(*d),
            DocNode::Concat(a, b) => self
                .simple_length_doc(*a)
                .and_then(|size1| self.simple_length_doc(*b).map(|size2| size1 + size2)),
            // Width of the narrowest single-line branch
            DocNode::Alt(a, b) => match (self.simple_length_doc(*a), self.simple_length_doc(*b)) {
                (Some(size1), Some(size2)) => Some(size1.min(size2)),
                (size1, size2) => size1.or(size2),
            },
        }
    }

    pub fn simple_length_frag(&self, frag: &Frag) -> Option<usize> {
        frag.branches.iter().map(|b| self.simple_length_doc(b.doc_id)).max().unwrap_or_default()
    }

    pub fn is_short(&self, doc_id: DocId) -> bool {
        self.simple_length_doc(doc_id).is_some_and(|size| size <= 10)
    }

    pub fn check_fn_call_width(&self, docs: &[DocId]) -> bool {
        if docs.is_empty() {
            return true;
        }
        let Some(mut length): Option<usize> = docs.iter().map(|doc| self.simple_length_doc(*doc)).sum()
        else {
            return false;
        };
        length += (docs.len() - 1) * 2;
        length < 60
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Edge {
    Nothing,
    Word,
    Keyword,   // let — always followed by a space
    Open,      // ( [ {
    Close,     // ) ] }
    BraceOpen,  // { of a block or struct literal
    BraceClose, // } of a block or struct literal
    Comma,     // , ;
    Tight,     // . :: ! & * # — nelepí mezeru
    Operator,  // + == => : ->
    Break,
}

/// One alternative of a fragment together with its edges
#[derive(Clone, Copy, Debug)]
pub struct Branch {
    pub doc_id: DocId,
    pub start: Edge,
    pub end: Edge,
}

type Branches = SmallVec<[Branch; 1]>;

#[derive(Clone, Debug)]
pub struct Frag {
    /// Alternatives of the fragment; never empty, at most one branch per (start, end) pair.
    /// Spaces around the fragment are placed inside the branches, so each branch gets
    /// the space that fits its own edges.
    branches: Branches,
    //empty_lines: u32,
    token: Option<SyntaxToken>,
}

impl Frag {
    pub fn new(doc_id: DocId, start: Edge, end: Edge, token: Option<SyntaxToken>) -> Self {
        Frag {
            branches: smallvec![Branch { start, doc_id, end }],
            token,
        }
    }

    pub fn new_simple(doc_id: DocId, edge: Edge, tokem: Option<SyntaxToken>) -> Self {
        Frag::new(doc_id, edge, edge, tokem)
    }

    pub fn empty() -> Self {
        Frag::new(EMPTY_DOC, Nothing, Nothing, None)
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

fn count_newlines(token: &SyntaxToken) -> u32 {
    token.text().bytes().filter(|c| *c == b'\n').count() as u32
}

/// Puts a single space after the opening and before the closing sigil of a single-line
/// block comment (`/*a*/` -> `/* a */`). Returns `None` when the text stays as it is.
/// Doc comments and empty comments are left alone.
fn normalize_block_comment(text: &str) -> Option<String> {
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

fn compute_space(left: Edge, right: Edge) -> DocId {
    use Edge::*;
    match (left, right) {
        (Break, _) | (_, Break) => NEWLINE_DOC,
        (Nothing, _) | (_, Nothing) => EMPTY_DOC,
        (BraceOpen, BraceClose)     => EMPTY_DOC,   // {}
        (_, Comma)                  => EMPTY_DOC,   // x,
        (Comma, Close)              => EMPTY_DOC,   // (x,)
        (Comma, _)                  => SPACE_DOC,    // , x
        (BraceOpen, _) | (_, BraceClose) => SPACE_DOC, // { x }
        (Open, _) | (_, Close)      => EMPTY_DOC,   // (x)
        (Keyword, _)                => SPACE_DOC,    // let (a, b)  let &x
        (Tight, _) | (_, Tight)     => EMPTY_DOC,   // a.b  a::b  &x
        (_, BraceOpen)              => SPACE_DOC,    // f() {
        (Operator, _) | (_, Operator) => SPACE_DOC,  // a + b
        (Word, Open)                => EMPTY_DOC,   // foo(
        (Close, Open)               => EMPTY_DOC,   // )(
        (Word, Word)                => SPACE_DOC,    // fn foo
        (Close, Word) | (Word, Close) => SPACE_DOC,
        _ => SPACE_DOC,
    }
}

pub(crate) struct Builder<'a> {
    fmt: &'a Formatter<'a>,
    frag: Frag,
}

impl<'a> Builder<'a> {
    pub fn new(fmt: &'a Formatter) -> Self {
        Builder {
            fmt,
            frag: Frag::empty(),
        }
    }

    /*pub fn new_with(fmt: &'a Formatter, frag: Frag) -> Self {
        Builder {
            fmt, frag, empty_lines: 0,
        }
    }*/

    #[inline]
    pub fn finish_node(mut self, node: &SyntaxNode) -> Frag {
        //self.push_node_prologue(node);
        //self.frag.element = Some(SyntaxElement::Node(node.clone()));
        self.frag
    }

    #[inline]
    pub fn to_frag(self) -> Frag {
        self.frag
    }

    pub fn is_empty(&self) -> bool {
        self.frag.is_empty()
    }

    /// Appends line-breaking `doc_id` to the branches whose end edge satisfies `pred`; they then end with nothing
    pub fn push_line_break_if(&mut self, doc_id: DocId, pred: impl Fn(Edge) -> bool) {
        let fmt = self.fmt;
        let frag = std::mem::take(&mut self.frag);
        self.frag = fmt.map_branches(frag, |b| {
            if pred(b.end) {
                Branch { doc_id: fmt.concat(b.doc_id, doc_id), end: Nothing, ..b }
            } else {
                b
            }
        });
    }

    pub fn set_token(&mut self, token: SyntaxToken) {
        self.frag.token = Some(token);
    }

    pub fn get_token(&self) -> Option<&SyntaxToken> {
        self.frag.token.as_ref()
    }

    pub fn push_newline(&mut self) {
        self.push_line_break_if(NEWLINE_DOC, |_| true);
        //self.frag.empty_lines = self.frag.empty_lines.saturating_sub(1);
    }

    /// Ends the line; blank lines from the source are kept
    pub fn ensure_newline(&mut self) {
        self.flush_trivia(true);
        if self.frag.is_empty() {
            self.push_newline();
        } else {
            self.push_line_break_if(NEWLINE_DOC, |end| end != Nothing);
        }
    }

    fn push_emptyline(&mut self) {
        self.push_line_break_if(EMPTYLINE_DOC, |_| true);
        //self.frag.empty_lines = 0;
    }

    fn flush_empty_lines(&mut self, newlines: u32) {
        if newlines == 0 {
            return;
        }
        let mut blank = (newlines - 1).min(self.fmt.config.blank_lines_upper_bound);
        if blank == 0 {
            // A single source newline is significant only after a line comment;
            // otherwise the caller decides spacing
            self.push_line_break_if(NEWLINE_DOC, |end| end == Break);
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

    /// Consumes the comments after the cursor (`frag.token`) and moves the cursor to the last consumed token.
    /// The whitespace after the cursor is left for `flush_trivia`.
    fn consume_comments(&mut self) {
        if let Some(token) = self.frag.token.take() {
            self.frag.token = Some(self.scan_comments(token, None));
        }
    }

    /// Pushes the comments following `token`; the source `delimiter`, if found among them, is consumed
    /// without printing. Returns the last consumed token.
    fn scan_comments(&mut self, token: SyntaxToken, mut delimiter: Option<SyntaxKind>) -> SyntaxToken {
        let mut cursor = token;
        let mut newlines = 0;
        let mut element = cursor.next_token();
        while let Some(token) = element {
            match token.kind() {
                SyntaxKind::WHITESPACE => {
                    newlines += count_newlines(&token);
                }
                SyntaxKind::COMMENT => {
                    let own_line = newlines > 0;
                    // Blank lines at the start of a fragment are dropped; the parent decides spacing
                    if newlines > 0 && !self.frag.is_empty() {
                        self.flush_empty_lines(newlines);
                    }
                    newlines = 0;
                    if own_line {
                        self.push_line_break_if(NEWLINE_DOC, |end| end != Nothing);
                    }
                    let end = if token.text().starts_with("//") { Break } else { Word };
                    let normalized = self.fmt.config.normalize_comments.then(|| normalize_block_comment(token.text())).flatten();
                    let doc_id = self.fmt.make_node(match normalized {
                        Some(text) => DocNode::Text(text),
                        None => DocNode::Token(token.clone()),
                    });
                    self.push(Frag::new(doc_id, Word, end, None));
                    cursor = token.clone();
                }
                x if Some(x) == delimiter => {
                    // The delimiter is moved before the comments, so newlines in front of it do not count
                    newlines = 0;
                    delimiter = None;
                    cursor = token.clone();
                }
                _ => break,
            }
            element = token.next_token();
        }
        cursor
    }

    /// Consumes the trivia after the cursor and clears it; blank lines at the end are kept
    /// only with `respect_newlines`
    pub fn flush_trivia(&mut self, respect_newlines: bool) {
        self.consume_comments();
        let Some(cursor) = self.frag.token.take() else {
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

    /*pub fn prepend(&mut self, mut frag: Frag) {
        std::mem::swap(&mut self.frag, &mut frag);
        self.push(frag);
    }*/

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
        let token = self.frag.token.take();
        if emit {
            let doc_id = match kind {
                SyntaxKind::SEMICOLON => SEMICOLON_DOC,
                SyntaxKind::COMMA => COMMA_DOC,
                _ => unreachable!(),
            };
            self.push(Frag::new_simple(doc_id, Comma, None));
        }
        if let Some(token) = token {
            self.frag.token = Some(self.scan_comments(token, Some(kind)));
        }
    }

    pub fn push(&mut self, frag: Frag) {
        if frag.is_empty() {
            return
        }
        if self.frag.is_empty() {
            self.frag = frag;
            return;
        }
        self.flush_trivia(false);
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
        self.frag.token = frag.token
    }
}