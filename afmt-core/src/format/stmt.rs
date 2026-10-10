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
use crate::layout::head::HeadLayouts;
use crate::layout::list::Delimiters;
use ra_ap_syntax::ast::{BlockExpr, Expr, HasAttrs, LetStmt, Stmt, StmtList};
use ra_ap_syntax::{AstNode, SyntaxKind};

fn format_let_stmt(fmt: &Formatter, stmt: LetStmt) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_keyword(stmt.let_token().unwrap());
    // Broken only when nothing else helps (Style Guide: break after `=`, then before `else`)
    builder.push(fmt.keep_together(format_pat(fmt, stmt.pat().unwrap())));
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

/// `init else {` of `let pat = init else { ... };`, `init` after `=` (Style Guide "else blocks
/// (let-else statements)"). `else {` follows the initializer on its line unless the initializer ends
/// with an indented continuation line (a broken chain, or the initializer moved to the line after
/// `=`); then it goes on its own line. It also goes there, at no cost, after an initializer on a
/// single line on the line of `=`: "If the let statement components preceding the else can be
/// formatted on a single line, but the else { does not fit on the same line, break the line before
/// the else."
fn format_init_with_else(fmt: &Formatter, init: Expr, else_block: Frag) -> Frag {
    let init = format_head_expr(fmt, init);
    let (same_line, next_line) = match init.other {
        Some(other) => {
            let (same_line, next_line) = fmt.same_and_indented_line(other);
            (Some(same_line), Some(next_line))
        }
        None => (None, None),
    };
    let hanging = init.hanging.map(|h| fmt.same_or_indented_line(h));
    let flat = same_line.clone().map(|same_line| fmt.flat(same_line));
    let placed = HeadLayouts {
        hanging: [hanging, next_line, flat]
            .into_iter()
            .flatten()
            .reduce(|a, b| fmt.alt_frag(a, b)),
        other: same_line,
    };
    format_head_followed_by(fmt, &placed, else_block)
}

fn format_stmt(fmt: &Formatter, stmt: Stmt) -> Frag {
    match stmt {
        Stmt::ExprStmt(expr) => {
            let mut builder = Builder::new(fmt);

            builder.push(format_stmt_expr(fmt, expr.expr().unwrap()));
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

/// The expression of an expression statement. A standalone `if` is never on a single line, nor is a
/// block other than `unsafe { ... }` (Style Guide "Blocks": a single-line block is "either used in
/// expression position (not statement position) or is an unsafe block in statement position"). The
/// tail expression of a block is its value, so it is in expression position, as in rustfmt.
fn format_stmt_expr(fmt: &Formatter, expr: Expr) -> Frag {
    match expr {
        Expr::IfExpr(expr) => format_if_expr(fmt, expr, false),
        Expr::BlockExpr(block) if block.unsafe_token().is_none() => format_block_expr(fmt, block),
        expr => format_expr(fmt, expr),
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
