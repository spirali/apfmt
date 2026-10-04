use crate::config::Config;
use crate::doc::DocNode::Newline;
use crate::doc::{
    COMMA_DOC, DocArena, DocId, DocNode, EMPTY_DOC, EMPTYLINE_DOC, NEWLINE_DOC,
    NEWLINE_OR_SPACE_DOC, SEMICOLON_DOC, SPACE_DOC,
};
use crate::resolver::Resolver;
use ra_ap_syntax::{
    AstNode, Direction, PreorderWithTokens, RustLanguage, SourceFile, SyntaxElement, SyntaxKind,
    SyntaxNode, SyntaxToken, WalkEvent, ast,
};
use std::cell::RefCell;

use crate::comments::{multiline_comment_lines, normalize_block_comment};
use crate::formatter::Edge::{
    BraceClose, BraceOpen, Break, Close, Comma, Keyword, Nothing, Open, Operator, Tight, Word,
};
use ra_ap_syntax::ast::{
    ArgList, BlockExpr, CallExpr, Comment, Expr, Fn as AstFn, HasArgList, HasModuleItem, HasName,
    Item, Literal, Name, ParamList, Path, PathExpr, PathSegment, Stmt, StmtList,
};
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

    fn make_node(&self, doc_node: DocNode) -> DocId {
        self.doc_arena.borrow_mut().make(doc_node)
    }

    pub fn token(&self, token: SyntaxToken, edge: Edge) -> Frag {
        Frag::new_simple(
            self.make_node(DocNode::Token(token.clone())),
            edge,
            Some(token),
        )
    }

    /// Just the token; the caller is responsible for the trivia (comments) that follow it
    pub fn token_no_follows(&self, token: SyntaxToken, edge: Edge) -> Frag {
        Frag::new_simple(self.make_node(DocNode::Token(token)), edge, None)
    }

    /// Keyword that is always followed by a space, e.g. `let (a, b)`
    pub fn keyword(&self, token: SyntaxToken) -> Frag {
        Frag::new(
            self.make_node(DocNode::Token(token.clone())),
            Word,
            Keyword,
            Some(token),
        )
    }

    fn concat(&self, doc1: DocId, doc2: DocId) -> DocId {
        if doc1 == EMPTY_DOC {
            return doc2;
        }
        if doc2 == EMPTY_DOC {
            return doc1;
        }
        self.make_node(DocNode::Concat(doc1, doc2))
    }

    pub fn nest(&self, frag: Frag) -> Frag {
        self.map_branches(frag, |b| Branch {
            doc_id: self.make_node(DocNode::Nest(b.doc_id)),
            ..b
        })
    }

    /// Puts `sep` (usually a line break) in front of every branch. The fragment then starts
    /// with `Nothing`, so no space is computed in front of it.
    fn prefix_break(&self, sep: DocId, frag: Frag) -> Frag {
        self.map_branches(frag, |b| Branch {
            start: Nothing,
            doc_id: self.concat(sep, b.doc_id),
            ..b
        })
    }

    /// The fragment starts on a new line with the current indentation
    pub fn on_next_line(&self, frag: Frag) -> Frag {
        self.prefix_break(NEWLINE_DOC, frag)
    }

    /// The fragment continues the current line after a space, or starts a new line
    pub fn on_same_or_next_line(&self, frag: Frag) -> Frag {
        self.prefix_break(NEWLINE_OR_SPACE_DOC, frag)
    }

    /// The fragment starts on a new line, indented one level deeper. The newline is inside
    /// the nest, otherwise the resolver measures the first nested line (and aligns multi-line
    /// comments) with the outer indent.
    pub fn indented_line(&self, frag: Frag) -> Frag {
        self.nest(self.on_next_line(frag))
    }

    /// The fragment continues the current line, or starts an indented next line,
    /// e.g. the right-hand side of `=`
    pub fn same_or_indented_line(&self, frag: Frag) -> Frag {
        self.alt_frag(frag.clone(), self.indented_line(frag))
    }

    /// Restricts the fragment to layouts that span more than one line
    pub fn multi_line(&self, frag: Frag) -> Frag {
        self.map_branches(frag, |b| Branch {
            doc_id: self.make_node(DocNode::MultiLine(b.doc_id)),
            ..b
        })
    }

    /// Restricts the fragment to layouts on a single line
    pub fn flat(&self, frag: Frag) -> Frag {
        self.map_branches(frag, |b| Branch {
            doc_id: self.make_node(DocNode::Flat(b.doc_id)),
            ..b
        })
    }

    /// Single-line layout of `frag`, if its width is at most `max_width`
    pub fn flat_within(&self, frag: Frag, max_width: u32) -> Option<Frag> {
        self.simple_length_frag(&frag)
            .is_some_and(|size| size as u32 <= max_width)
            .then(|| self.flat(frag))
    }

    fn alt_doc(&self, doc1: DocId, doc2: DocId) -> DocId {
        if doc1 == doc2 {
            return doc1;
        }
        self.make_node(DocNode::Alt(doc1, doc2))
    }

    pub fn alt_opt(&self, frag: Frag, other: Option<Frag>) -> Frag {
        match other {
            Some(other) => self.alt_frag(frag, other),
            None => frag,
        }
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

    fn map_branches(&self, mut frag: Frag, f: impl FnMut(Branch) -> Branch) -> Frag {
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
            if let Some(o) = out
                .iter_mut()
                .find(|o| o.start == b.start && o.end == b.end)
            {
                o.doc_id = self.alt_doc(o.doc_id, b.doc_id);
            } else {
                out.push(b);
            }
        }
        out
    }

    /// Doc of the whole fragment; the edges are dropped
    fn frag_to_doc(&self, frag: &Frag) -> DocId {
        frag.branches
            .iter()
            .map(|b| b.doc_id)
            .reduce(|a, b| self.alt_doc(a, b))
            .unwrap()
    }

    pub fn dump(&self, frag: &Frag) {
        let root = self.frag_to_doc(frag);
        self.doc_arena.borrow().dump(root);
    }

    pub fn into_resolver(self, root: Frag) -> Resolver<'a> {
        let root = self.frag_to_doc(&root);
        Resolver::new(self.doc_arena.into_inner(), self.config, root)
    }

    fn comment_doc(&self, token: &SyntaxToken) -> DocId {
        if let Some(lines) = multiline_comment_lines(token) {
            // The lines are aligned to the column of the comment start
            let mut lines = lines.into_iter();
            let mut doc_id = self.make_node(DocNode::Text(lines.next().unwrap().to_string()));
            for line in lines {
                doc_id = self.concat(doc_id, NEWLINE_DOC);
                if !line.is_empty() {
                    doc_id = self.concat(doc_id, self.make_node(DocNode::Text(line.to_string())));
                }
            }
            return self.make_node(DocNode::Align(doc_id));
        }
        let normalized = self
            .config
            .normalize_comments
            .then(|| normalize_block_comment(token.text()))
            .flatten();
        self.make_node(match normalized {
            Some(text) => DocNode::Text(text),
            None => DocNode::Token(token.clone()),
        })
    }

    fn simple_length_doc(&self, doc_id: DocId) -> Option<usize> {
        match self.doc_arena.borrow().get_node(doc_id) {
            DocNode::Token(token) => Some(unicode_width::UnicodeWidthStr::width(token.text())),
            DocNode::Text(t) => Some(unicode_width::UnicodeWidthStr::width(t.as_str())),
            Newline | DocNode::Nest(_) | DocNode::Align(_) => None,
            DocNode::Flat(d) => self.simple_length_doc(*d),
            DocNode::MultiLine(_) => None,
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

    /// Width of the fragment on a single line; `None` if it cannot be on a single line
    pub fn simple_length_frag(&self, frag: &Frag) -> Option<usize> {
        frag.branches
            .iter()
            .map(|b| self.simple_length_doc(b.doc_id))
            .max()
            .unwrap_or_default()
    }
}

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
}

/// One alternative of a fragment together with its edges
#[derive(Clone, Copy, Debug)]
struct Branch {
    doc_id: DocId,
    start: Edge,
    end: Edge,
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
    fn new(doc_id: DocId, start: Edge, end: Edge, token: Option<SyntaxToken>) -> Self {
        Frag {
            branches: smallvec![Branch { start, doc_id, end }],
            token,
        }
    }

    fn new_simple(doc_id: DocId, edge: Edge, tokem: Option<SyntaxToken>) -> Self {
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

/// Start edges of fragments that continue the construct on their left: everything except closing
/// delimiters and the `{` of a block. Own-line comments in front of them are indented together with them.
fn is_continuation(edge: Edge) -> bool {
    !matches!(edge, Close | BraceClose | BraceOpen)
}

/// An own-line block comment directly followed by code on the same line, e.g. `/* c */ f();`
fn is_leading_block_comment(token: &SyntaxToken) -> bool {
    let has_newline = |t: Option<SyntaxToken>| {
        t.is_some_and(|t| t.kind() == SyntaxKind::WHITESPACE && count_newlines(&t) > 0)
    };
    token.kind() == SyntaxKind::COMMENT
        && token.text().starts_with("/*")
        && has_newline(token.prev_token())
        && !has_newline(token.next_token())
}

/// Where `Builder::scan_comments` stops at the latest
#[derive(Clone, Copy, PartialEq, Eq)]
enum ScanStop {
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

fn compute_space(left: Edge, right: Edge) -> DocId {
    use Edge::*;
    match (left, right) {
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
        (Close, Open) => EMPTY_DOC,                    // )(
        (Word, Word) => SPACE_DOC,                     // fn foo
        (Close, Word) | (Word, Close) => SPACE_DOC,
        _ => SPACE_DOC,
    }
}

#[derive(Clone)]
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

    pub fn with_token(fmt: &'a Formatter, token: Option<SyntaxToken>) -> Self {
        let mut frag = Frag::empty();
        frag.token = token;
        Builder { fmt, frag }
    }

    #[inline]
    pub fn to_frag(self) -> Frag {
        self.frag
    }

    pub fn is_empty(&self) -> bool {
        self.frag.is_empty()
    }

    /// Appends line-breaking `doc_id` to the branches whose end edge satisfies `pred`; they then end with nothing
    fn push_line_break_if(&mut self, doc_id: DocId, pred: impl Fn(Edge) -> bool) {
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

    pub fn get_token(&self) -> Option<&SyntaxToken> {
        self.frag.token.as_ref()
    }

    /// Ends the line after a trailing line comment, which would otherwise swallow the following code
    pub fn push_break_after_line_comment(&mut self) {
        self.push_line_break_if(NEWLINE_DOC, |end| end == Break);
    }

    pub fn push_newline(&mut self) {
        self.push_line_break_if(NEWLINE_DOC, |_| true);
        //self.frag.empty_lines = self.frag.empty_lines.saturating_sub(1);
    }

    /// Pushes `frag` on a new line; the first item of the builder stays where it is.
    /// Blank lines from the source in front of it are kept.
    pub fn push_on_new_line(&mut self, frag: Frag) {
        if !self.is_empty() {
            self.ensure_newline();
        }
        self.push(frag);
    }

    /// Ends the line; blank lines from the source are kept
    pub fn ensure_newline(&mut self) {
        self.consume_comments();
        // The comment belongs to the following code, so no line break after it
        let leading_comment = self
            .frag
            .token
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
            self.push_break_after_line_comment();
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
    pub fn consume_comments(&mut self) {
        if let Some(token) = self.frag.token.take() {
            self.frag.token = Some(self.scan_comments(token, None, ScanStop::Code));
        }
    }

    /// Pushes the comments following `token`; the source `delimiter`, if found among them, is consumed
    /// without printing. `stop` says where the scan ends at the latest.
    /// Returns the last consumed token.
    fn scan_comments(
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
                    let end = if token.text().starts_with("//") {
                        Break
                    } else {
                        Word
                    };
                    let doc_id = self.fmt.comment_doc(&token);
                    self.push(Frag::new(doc_id, Edge::Comment, end, None));
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

    /// Starts a vertical list after the opening delimiter (the cursor): comments on its line stay
    /// there, then a line break, then the other comments. A line break after the comments
    /// is kept when the source has one.
    pub fn flush_leading_comments(&mut self) {
        if let Some(token) = self.frag.token.take() {
            self.frag.token = Some(self.scan_comments(token, None, ScanStop::OwnLine));
        }
        self.push_newline();
        self.consume_comments();
        let Some(cursor) = self.frag.token.take() else {
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
            self.push(Frag::new_simple(delimiter_doc(kind), Comma, None));
        }
        if let Some(token) = token {
            self.frag.token = Some(self.scan_comments(token, Some(kind), ScanStop::Code));
        }
    }

    /// Consumes the delimiter `kind` after a list item (the cursor) together with the item's post
    /// comments, as rustfmt calls them: the comments in front of the delimiter and the comments after
    /// it that end the line. Comments followed by code on the same line and own-line comments are
    /// pre comments of the next item, so they are left for `push_pre_comments`.
    /// Returns whether a post comment was pushed.
    pub fn push_delimiter_with_post_comments(&mut self, kind: SyntaxKind) -> bool {
        let token = self.frag.token.take();
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
        self.frag.token = Some(cursor);
        had_comment
    }

    /// Pushes the comments in front of the next list item (rustfmt's pre comments). A line break
    /// follows them when the source has one after them, otherwise the item follows after a space.
    pub fn push_pre_comments(&mut self) {
        self.consume_comments();
        let Some(cursor) = self.frag.token.as_ref() else {
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
        let Some(token) = self.frag.token.take() else {
            return;
        };
        if rest_of_line_is_comments(&token, None) {
            self.frag.token = Some(self.scan_comments(token, None, ScanStop::OwnLine));
            self.push_line_break_if(EMPTY_DOC, |end| end == Break);
        } else {
            self.frag.token = Some(token);
        }
    }

    /// Moves the cursor out of the builder; the trivia after it is processed by another builder
    pub fn take_cursor(&mut self) -> Option<SyntaxToken> {
        self.frag.token.take()
    }

    /// A space or a line break, whichever gives the better layout
    pub fn push_soft_line_break(&mut self) {
        self.push_line_break_if(NEWLINE_OR_SPACE_DOC, |_| true);
    }

    pub fn has_own_line_comment(&mut self) -> bool {
        let Some(token) = self.frag.token.as_ref() else {
            return false;
        };
        let mut token = token.next_token();
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
        if self.frag.token.is_some() && frag.branches.iter().all(|b| is_continuation(b.start)) {
            self.push_continuation(frag);
            return;
        }
        self.flush_trivia(false);
        self.join(frag);
    }

    fn push_continuation(&mut self, frag: Frag) {
        // Comments on the same line as the left side stay there
        let token = self.frag.token.take().unwrap();
        self.frag.token = Some(self.scan_comments(token, None, ScanStop::OwnLine));
        if !self.has_own_line_comment() {
            self.flush_trivia(false);
            self.join(frag);
            return;
        }
        let fmt = self.fmt;
        let mut nested = Builder::with_token(fmt, self.frag.token.take());
        nested.flush_trivia(false);
        nested.push(frag);
        // A line comment on the left side is ended by the nested newline
        self.push_line_break_if(EMPTY_DOC, |end| end == Break);
        self.join(fmt.indented_line(nested.to_frag()));
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
        self.frag.token = frag.token
    }
}
