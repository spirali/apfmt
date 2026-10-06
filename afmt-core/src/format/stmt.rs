//! Statements and blocks

use super::expr::{format_expr, format_if_expr};
use super::item::format_item;
use super::pat::format_pat;
use super::ty::format_type;
use crate::formatter::Edge::{BraceClose, BraceOpen, Comma, Operator, Word};
use crate::formatter::{Builder, Formatter, Frag};
use ra_ap_syntax::ast::{BlockExpr, Expr, LetStmt, Stmt, StmtList};
use ra_ap_syntax::SyntaxKind;

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
        builder.push(fmt.same_or_indented_line(format_expr(fmt, init)));
    }
    if let Some(let_else) = stmt.let_else() {
        builder.push_token(let_else.else_token().unwrap(), Word);
        builder.push(format_block_expr(fmt, let_else.block_expr().unwrap()));
    }
    builder.push_delimiter(SyntaxKind::SEMICOLON);
    builder.into_frag()
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
                builder.push_delimiter(SyntaxKind::SEMICOLON);
            }
            builder.into_frag()
        }
        Stmt::Item(item) => format_item(fmt, item),
        Stmt::LetStmt(stmt) => format_let_stmt(fmt, stmt),
    }
}

/// `tail` is the already formatted tail expression, when it is shared with another layout
fn format_stmt_list(fmt: &Formatter, stmt_list: StmtList, tail: Option<Frag>) -> Frag {
    let l_curly = stmt_list.l_curly_token().unwrap();
    // Tokens following `{` belong to the body, so they are nested with it; blank lines right after
    // `{` are dropped
    let mut builder = Builder::with_cursor(fmt, Some(l_curly.clone()));
    let line_comments = builder.push_comments_ending_line();
    builder.consume_comments();
    for child in stmt_list.statements() {
        builder.push_on_new_line(format_stmt(fmt, child));
    }
    if let Some(expr) = stmt_list.tail_expr() {
        builder.push_on_new_line(tail.unwrap_or_else(|| format_expr(fmt, expr)));
    }

    // Blank lines before `}` are dropped
    builder.flush_trivia(false);
    let body_frag = if !builder.is_empty() {
        builder.push_newline();
        fmt.body_after_opener(line_comments, builder.into_frag())
    } else {
        Frag::empty()
    };
    let mut builder = Builder::new(fmt);
    builder.push(fmt.token_without_cursor(l_curly, BraceOpen));
    builder.push(body_frag);
    builder.push_token(stmt_list.r_curly_token().unwrap(), BraceClose);
    builder.into_frag()
}

pub(super) fn format_block_expr(fmt: &Formatter, body: BlockExpr) -> Frag {
    format_block_expr_with_tail(fmt, body, None)
}

/// See `format_stmt_list` for `tail`
pub(super) fn format_block_expr_with_tail(
    fmt: &Formatter,
    body: BlockExpr,
    tail: Option<Frag>,
) -> Frag {
    format_stmt_list(fmt, body.stmt_list().unwrap(), tail)
}
