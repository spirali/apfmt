//! Statements and blocks

use super::attr::{format_attr, format_with_attrs};
use super::expr::{format_expr, format_head_expr, format_head_followed_by, format_if_expr};
use super::format_unsupported;
use super::item::format_item;
use super::pat::format_pat;
use super::ty::format_type;
use crate::format::common::{push_label, push_modifiers};
use crate::formatter::Edge::{Comma, Operator, Word};
use crate::formatter::{Builder, Formatter, Frag};
use crate::layout::list::Delimiters;
use ra_ap_syntax::ast::{BlockExpr, Expr, HasAttrs, LetStmt, Stmt, StmtList};
use ra_ap_syntax::{AstNode, SyntaxKind};

fn format_let_stmt(fmt: &Formatter, stmt: LetStmt) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_keyword(stmt.let_token().unwrap());
    builder.push(format_pat(fmt, stmt.pat().unwrap()));
    if let Some(ty) = stmt.ty() {
        builder.push_token(stmt.colon_token().unwrap(), Comma);
        builder.push(format_type(fmt, ty));
    }
    if let Some(init) = stmt.initializer() {
        builder.push_token(stmt.eq_token().unwrap(), Operator);
        match stmt.let_else() {
            Some(let_else) => {
                let mut else_block = Builder::new(fmt);
                else_block.push_token(let_else.else_token().unwrap(), Word);
                else_block.push(format_block_expr(fmt, let_else.block_expr().unwrap()));
                builder.push(format_init_with_else(fmt, init, else_block.into_frag()));
            }
            None => builder.push(fmt.same_or_indented_line(format_expr(fmt, init))),
        }
    }
    builder.push_separator(SyntaxKind::SEMICOLON);
    builder.into_frag()
}

/// `init else {` of `let pat = init else { ... };`, `init` after `=`. `else {`
/// follows the initializer on its line unless the initializer ends with an indented continuation line
/// (a broken chain); then it goes on its own line (Style Guide "else blocks (let-else statements)").
fn format_init_with_else(fmt: &Formatter, init: Expr, else_block: Frag) -> Frag {
    let init = format_head_expr(fmt, init);
    format_head_followed_by(fmt, &init, else_block, |init| {
        fmt.same_or_indented_line(init)
    })
}

fn format_stmt(fmt: &Formatter, stmt: Stmt) -> Frag {
    match stmt {
        Stmt::ExprStmt(expr) => {
            let mut builder = Builder::new(fmt);

            let inner = match expr.expr().unwrap() {
                // A standalone `if` is never put on a single line
                Expr::IfExpr(if_expr) => format_if_expr(fmt, if_expr, false),
                e => format_expr(fmt, e),
            };
            builder.push(inner);
            // Block-like expressions (`if`, ...) may stand without `;`
            if expr.semicolon_token().is_some() {
                builder.push_separator(SyntaxKind::SEMICOLON);
            }
            builder.into_frag()
        }
        Stmt::Item(item) => format_item(fmt, item),
        Stmt::LetStmt(stmt) => {
            format_with_attrs(fmt, &stmt, false, || format_let_stmt(fmt, stmt.clone()))
        }
    }
}

/// `tail` is the already formatted tail expression, when it is shared with another layout
fn format_stmt_list(fmt: &Formatter, stmt_list: StmtList, tail: Option<Frag>) -> Frag {
    let delims = Delimiters::braces_of(stmt_list.syntax());
    let tail = stmt_list
        .tail_expr()
        .map(|expr| tail.unwrap_or_else(|| format_expr(fmt, expr)));
    let lines = stmt_list
        .attrs()
        .map(|attr| format_attr(fmt, attr))
        .chain(stmt_list.statements().map(|stmt| format_stmt(fmt, stmt)))
        .chain(tail);
    delims.format_block(fmt, lines)
}

pub(super) fn format_block_expr(fmt: &Formatter, body: BlockExpr) -> Frag {
    format_block_expr_with_tail(fmt, body, None)
}

/// `{ ... }` with its label and modifiers: `'a: {`, `unsafe {`, `async move {`, `const {`, `gen {`;
/// see `format_stmt_list` for `tail`
pub(super) fn format_block_expr_with_tail(
    fmt: &Formatter,
    body: BlockExpr,
    tail: Option<Frag>,
) -> Frag {
    if body.try_block_modifier().is_some() {
        return format_unsupported(fmt, body.syntax(), "a `try` block");
    }
    let mut builder = Builder::new(fmt);
    push_block_prefix(fmt, &mut builder, &body);
    builder.push(format_stmt_list(fmt, body.stmt_list().unwrap(), tail));
    builder.into_frag()
}

/// Pushes the label and the modifiers of a block in front of its `{`: `'a: `, `unsafe `, `async move `
pub(super) fn push_block_prefix(fmt: &Formatter, builder: &mut Builder, body: &BlockExpr) {
    push_label(fmt, builder, body.label());
    push_modifiers(builder, body.syntax());
}
