
use std::cell::RefCell;
use crate::config::Config;
use crate::doc::DocNode::Newline;
use crate::doc::{Doc, DocArena, DocId, DocNode, EMPTY_DOC, EMPTYLINE_DOC, NEWLINE_DOC, NEWLINE_OR_SPACE_DOC, SPACE_DOC, SEMICOLON_DOC};
use crate::resolver::Resolver;
use ra_ap_syntax::{ast, PreorderWithTokens, RustLanguage, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken, WalkEvent, AstNode, SourceFile, Direction};

use ra_ap_syntax::ast::{ArgList, BlockExpr, CallExpr, Comment, Expr, HasArgList, HasModuleItem, HasName, Item, Literal, Name, Path, PathExpr, PathSegment, Stmt, StmtList, Fn as AstFn, ParamList};
use crate::formatter::Edge::{Break, BraceClose, BraceOpen, Close, Comma, Nothing, Open, Operator, Tight, Word};

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

    fn make_node(&self, doc_node: DocNode) -> DocId {
        self.doc_arena.borrow_mut().make(doc_node)
    }

    /*pub fn text(&self, s: String) -> DocId {
        if s.is_empty() {
            return EMPTY_DOC;
        }
        self.make_node(DocNode::Text(s.into()))
    }

    pub fn s_text(&self, s: &str) -> DocId {
        if s.is_empty() {
            return EMPTY_DOC;
        }
        self.make_node(DocNode::Text(s.into()))
    }*/

    //fn process_after_token(&self, token: , frag: Frag)

    pub fn token(&self, token: SyntaxToken, edge: Edge) -> Frag {
        Frag::new_simple(self.make_node(DocNode::Token(token)), edge)
    }

    pub fn text(&self, text: String, edge: Edge) -> Frag {
        Frag::new_simple(self.make_node(DocNode::Text(text)), edge)
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

    pub fn nest(&self, mut frag: Frag) -> Frag {
        frag.doc_id = self.make_node(DocNode::Nest(frag.doc_id));
        frag
    }

    pub fn alt(&self, doc1: DocId, doc2: DocId) -> DocId {
        self.make_node(DocNode::Alt(doc1, doc2))
    }

    /*pub fn concat_text(&self, doc: DocId, text: String) -> DocId {
        let txt = self.text(text);
        self.concat(doc, txt)
    }

    pub fn concat_s_text(&self, doc: DocId, text: &str) -> DocId {
        let txt = self.text(text.to_string());
        self.concat(doc, txt)
    }*/

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

    pub fn simple_length(&self, doc_id: DocId) -> Option<usize> {
        match self.doc_arena.borrow().get_node(doc_id) {
            DocNode::Token(token) => Some(unicode_width::UnicodeWidthStr::width(token.text())),
            DocNode::Text(t) => Some(unicode_width::UnicodeWidthStr::width(t.as_str())),
            Newline | DocNode::Nest(_) | DocNode::Align(_) => None,
            DocNode::Concat(a, b) => self
                .simple_length(*a)
                .and_then(|size1| self.simple_length(*b).map(|size2| size1 + size2)),
            DocNode::Alt(a, b) => {
                if let Some(size1) = self.simple_length(*a) {
                    Some(if let Some(size2) = self.simple_length(*b) {
                        size1.max(size2)
                    } else {
                        size1
                    })
                } else {
                    None
                }
            }
        }
    }

    pub fn is_short(&self, doc_id: DocId) -> bool {
        self.simple_length(doc_id).is_some_and(|size| size <= 10)
    }

    pub fn check_fn_call_width(&self, docs: &[DocId]) -> bool {
        if docs.is_empty() {
            return true;
        }
        let Some(mut length): Option<usize> = docs.iter().map(|doc| self.simple_length(*doc)).sum()
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
    Open,      // ( [ {
    Close,     // ) ] }
    BraceOpen,  // { of a block or struct literal
    BraceClose, // } of a block or struct literal
    Comma,     // , ;
    Tight,     // . :: ! & * # — nelepí mezeru
    Operator,  // + == => : ->
    Break,
}

#[derive(Clone, Copy, Debug)]
pub struct Frag {
    doc_id: DocId,
    start: Edge,
    end: Edge,
    empty_lines: u32,
}

const EMPTY_FRAG: Frag = Frag {
    doc_id: EMPTY_DOC,
    start: Nothing,
    end: Nothing,
    empty_lines: 0,
};

impl Frag {

    pub fn new_from(doc_id: DocId, frag: &Frag) -> Self {
        Frag {
            doc_id,
            start: frag.start,
            end: frag.end,
            empty_lines: frag.empty_lines,
        }
    }

    pub fn new_simple(doc_id: DocId, edge: Edge) -> Self {
        Frag {
            doc_id,
            start: edge,
            end: edge,
            empty_lines: 0,
        }
    }

    fn reset_empty_lines(&mut self) {
        self.empty_lines = 0;
    }


    #[inline]
    pub fn is_empty(&self) -> bool {
        self.doc_id == EMPTY_DOC
    }
}

fn compute_space(left: Edge, right: Edge) -> DocId {
    use Edge::*;
    match (left, right) {
        (Break, _) | (_, Break) => NEWLINE_DOC,
        (Nothing, _) | (_, Nothing) => EMPTY_DOC,
        (BraceOpen, BraceClose)     => EMPTY_DOC,   // {}
        (_, Comma)                  => EMPTY_DOC,   // x,
        (Comma, _)                  => SPACE_DOC,    // , x
        (BraceOpen, _) | (_, BraceClose) => SPACE_DOC, // { x }
        (Open, _) | (_, Close)      => EMPTY_DOC,   // (x)
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


struct Builder<'a> {
    fmt: &'a Formatter<'a>,
    frag: Frag,
}

impl<'a> Builder<'a> {
    pub fn new(fmt: &'a Formatter) -> Self {
        Builder {
            fmt,
            frag: EMPTY_FRAG,
        }
    }

    /*pub fn new_with(fmt: &'a Formatter, frag: Frag) -> Self {
        Builder {
            fmt, frag, empty_lines: 0,
        }
    }*/

    #[inline]
    fn finish_node(mut self, node: &SyntaxNode) -> Frag {
        self.push_node_prologue(node);
        self.frag
    }

    #[inline]
    fn to_frag(self) -> Frag {
        self.frag
    }

    fn is_empty(&self) -> bool {
        self.frag.is_empty()
    }

    fn push_newline(&mut self) {
        self.frag.doc_id = self.fmt.concat(self.frag.doc_id, NEWLINE_DOC);
        self.frag.end = Nothing;
        self.frag.empty_lines = self.frag.empty_lines.saturating_sub(1);
    }

    fn ensure_newline(&mut self) {
        self.flush_empty_lines();
        if self.frag.end != Nothing || self.frag.is_empty() {
            self.push_newline();
        }
    }

    fn push_emptyline(&mut self) {
        self.frag.doc_id = self.fmt.concat(self.frag.doc_id, EMPTYLINE_DOC);
        self.frag.end = Nothing;
        self.frag.empty_lines = 0;
    }

    fn flush_empty_lines(&mut self) {
        let newlines = self.frag.empty_lines;
        if newlines == 0 {
            return;
        }
        self.frag.empty_lines = 0;
        let mut blank = (newlines - 1).min(self.fmt.config.blank_lines_upper_bound);
        if blank == 0 && self.frag.end != Break {
            // A single source newline is not significant; the caller decides spacing
            return;
        }
        blank += 1;
        while blank > 1 {
            self.frag.doc_id = self.fmt.concat(self.frag.doc_id, EMPTYLINE_DOC);
            blank -= 2;
        }
        if blank == 1 {
            self.frag.doc_id = self.fmt.concat(self.frag.doc_id, NEWLINE_DOC);
        }
        self.frag.end = Nothing;
    }

    fn prepend(&mut self, mut frag: Frag) {
        std::mem::swap(&mut self.frag, &mut frag);
        self.push(frag);
    }

    fn push(&mut self, frag: Frag) {
        if frag.is_empty() {
            return
        }
        if self.frag.is_empty() {
            self.frag = frag;
            return;
        }
        self.flush_empty_lines();
        let space = compute_space(self.frag.end, frag.start);
        if !space.is_empty() {
            self.frag.doc_id = self.fmt.concat(self.frag.doc_id, space);
        }
        self.frag.doc_id = self.fmt.concat(self.frag.doc_id, frag.doc_id);
        self.frag.end = frag.end;
        self.frag.empty_lines = frag.empty_lines
    }

    fn push_node_prologue(&mut self, node: &SyntaxNode) {
        self.process_tokens(node.siblings_with_tokens(Direction::Next).skip(1))
    }

    fn process_tokens(&mut self, items: impl Iterator<Item=SyntaxElement>) {
        for item in items {
            match item {
                SyntaxElement::Node(_) => {
                    break
                }
                SyntaxElement::Token(token) => {
                    match token.kind() {
                        SyntaxKind::WHITESPACE => {
                            self.frag.empty_lines += token.text().bytes().filter(|c| *c == b'\n').count() as u32;
                        }
                        SyntaxKind::COMMENT => {
                            let own_line = self.frag.empty_lines > 0;
                            self.flush_empty_lines();
                            if own_line && self.frag.end != Nothing {
                                self.push_newline();
                            }
                            let end = if token.text().starts_with("//") { Break } else { Word };
                            let doc_id = self.fmt.make_node(DocNode::Token(token));
                            self.push(Frag { doc_id, start: Word, end, empty_lines: 0 });
                        }
                        _ => break,
                    }
                }
            }
        }

    }
    // #[inline]
    // fn push_newline(&mut self) {
    //     self.push(NEWLINE_DOC);
    // }
}

fn format_name(fmt: &Formatter, name: Name) -> Frag {
    fmt.text(name.text().to_string(), Word)
}

fn format_path_expr(fmt: &Formatter, path_expr: PathExpr) -> Frag {
    format_path(fmt, path_expr.path().unwrap())
}

fn format_path(fmt: &Formatter, path: Path) -> Frag {
    fmt.text(path.syntax().text().to_string(), Word)
}

fn format_token(fmt: &Formatter, token: SyntaxToken, edge: Edge) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.process_tokens(token.siblings_with_tokens(Direction::Next).skip(1));
    builder.prepend(fmt.token(token, edge));
    builder.to_frag()
}

/// Formats just the token; the caller is responsible for the tokens that follow it
fn format_token_no_follows(fmt: &Formatter, token: SyntaxToken, edge: Edge) -> Frag {
    fmt.token(token, edge)
}

fn format_token_const(fmt: &Formatter, token: SyntaxToken, frag: Frag) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.process_tokens(token.siblings_with_tokens(Direction::Next).skip(1));
    builder.prepend(frag);
    builder.to_frag()
}


fn list_comma_separated(fmt: &Formatter, docs: &[Frag]) -> Frag {
    let mut it = docs.iter();
    let Some(frag) = it.next() else {
        return EMPTY_FRAG;
    };
    let mut builder = Builder::new(fmt);
    builder.push(*frag);
    for d in it {
        builder.push(fmt.text(", ".to_string(), Nothing));
        builder.push(*d);
    }
    builder.to_frag()
}

fn format_arg_list(fmt: &Formatter, arg_list: ArgList) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_token(fmt, arg_list.l_paren_token().unwrap(), Open));

    let args = arg_list.args().map(|a| format_expr(fmt, a)).collect::<Vec<Frag>>();

    builder.push(list_comma_separated(fmt, &args));

    builder.push(format_token(fmt, arg_list.r_paren_token().unwrap(), Close));
    builder.finish_node(arg_list.syntax())
}

fn format_call_expr(fmt: &Formatter, call: CallExpr) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_expr(fmt, call.expr().unwrap()));
    builder.push(format_arg_list(fmt, call.arg_list().unwrap()));
    builder.finish_node(call.syntax())
}

fn format_literal(fmt: &Formatter, literal: Literal) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_token(fmt, literal.token(), Word));
    builder.finish_node(literal.syntax())
}

fn format_expr(fmt: &Formatter, expr: Expr) -> Frag {
    match expr {
        Expr::Literal(expr) => format_literal(fmt, expr),
        Expr::CallExpr(expr) => format_call_expr(fmt, expr),
        Expr::PathExpr(expr) => format_path_expr(fmt, expr),
        x => {
            panic!{
                "{x:?}"
            }
        }
    }
}

fn format_stmt(fmt: &Formatter, stmt: Stmt) -> Frag {
    match stmt {
        Stmt::ExprStmt(expr) => {
            let mut builder = Builder::new(fmt);
            builder.push(format_expr(fmt, expr.expr().unwrap()));
            builder.push(format_token(fmt, expr.semicolon_token().unwrap(), Comma));
            builder.finish_node(expr.syntax())
        }
        Stmt::Item(_) => {
            todo!()
        }
        Stmt::LetStmt(_) => {
            todo!()
        }
    }
}

fn format_stmt_list(fmt: &Formatter, stmt_list: StmtList) -> Frag {
    let l_curly = stmt_list.l_curly_token().unwrap();
    let mut followers = l_curly.siblings_with_tokens(Direction::Next).skip(1).peekable();

    // Blank lines right after `{` are dropped; only whether the first element starts a new line matters
    let mut on_new_line = false;
    while let Some(ws) = followers.next_if(|e| e.kind() == SyntaxKind::WHITESPACE) {
        on_new_line |= ws.as_token().is_some_and(|t| t.text().contains('\n'));
    }
    // A comment on the line of `{` stays there if it fits
    let separator = if !on_new_line && followers.peek().is_some_and(|e| e.kind() == SyntaxKind::COMMENT) {
        NEWLINE_OR_SPACE_DOC
    } else {
        NEWLINE_DOC
    };

    // Tokens following `{` belong to the body, so they are nested with it
    let mut builder = Builder::new(fmt);
    builder.process_tokens(followers);
    for child in stmt_list.statements() {
        if !builder.is_empty() {
            builder.ensure_newline();
        }
        builder.push(format_stmt(fmt, child));
    }

    let body_frag = if !builder.is_empty() {
        builder.push_newline();
        let mut frag = builder.to_frag();
        frag.reset_empty_lines();
        frag.doc_id = fmt.concat(separator, frag.doc_id);
        frag.start = Nothing;
        fmt.nest(frag)
    } else {
        EMPTY_FRAG
    };
    let mut builder = Builder::new(fmt);
    builder.push(format_token_no_follows(fmt, l_curly, BraceOpen));
    builder.push(body_frag);
    builder.push(format_token(fmt, stmt_list.r_curly_token().unwrap(), BraceClose));
    builder.finish_node(stmt_list.syntax())
}

fn format_block_expr(fmt: &Formatter, body: BlockExpr) -> Frag {
    format_stmt_list(fmt, body.stmt_list().unwrap())
}

fn format_param_list(fmt: &Formatter, param_list: ParamList) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_token(fmt, param_list.l_paren_token().unwrap(), Open));
    builder.push(format_token(fmt, param_list.r_paren_token().unwrap(), Close));
    builder.finish_node(param_list.syntax())
}

fn format_fn(fmt: &Formatter, item_fn: AstFn) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_token(fmt, item_fn.fn_token().unwrap(), Word));
    builder.push(format_name(fmt, item_fn.name().unwrap()));
    builder.push(format_param_list(fmt, item_fn.param_list().unwrap()));
    builder.push(format_block_expr(fmt, item_fn.body().unwrap()));
    builder.finish_node(item_fn.syntax())
}

fn format_item(fmt: &Formatter, item: Item) -> Frag {
    let mut builder = Builder::new(fmt);
    match item {
        Item::Fn(item) => {
            builder.push(format_fn(fmt, item))
        },
        x => {
            panic!("{x:?}");
        }
    }
    builder.to_frag()
}

pub(crate) fn format_source_file(fmt: &Formatter, node: SourceFile) -> DocId {
    let mut builder = Builder::new(fmt);
    for child in node.items() {
        builder.push(format_item(fmt, child));
    }
    builder.to_frag().doc_id
}