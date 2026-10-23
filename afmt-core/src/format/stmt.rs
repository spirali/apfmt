//! Statements and blocks

use super::attr::{debug_assert_attrs_placed, format_attr, format_skippable_with_attrs};
use super::expr::{
    ExprPosition, format_braced_block, format_expr, format_expr_at, format_head_expr,
    format_head_followed_by, single_line_block_expr,
};
use super::format_unsupported;
use super::item::format_item;
use super::pat::format_pat;
use super::ty::format_type;
use crate::doc::SEMICOLON_DOC;
use crate::format::common::{push_label, push_modifiers};
use crate::formatter::Edge::{Comma, Operator, Word};
use crate::formatter::{Builder, Formatter, Frag};
use crate::layout::head::HeadLayouts;
use crate::layout::list::Delimiters;
use crate::syntax_utils::{is_jump_expr, trailing_jump_stmt};
use ra_ap_syntax::ast::{BlockExpr, ExprStmt, HasAttrs, LetStmt, Stmt, StmtList};
use ra_ap_syntax::{AstNode, SyntaxKind, SyntaxNode};

fn format_let_stmt(fmt: &Formatter, stmt: LetStmt) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_keyword(stmt.let_token().unwrap());
    // `pat: Type`, broken only when nothing else helps (Style Guide: break after `=`, then before
    // `else`)
    let mut lhs = Builder::new(fmt);
    lhs.push(fmt.keep_together(format_pat(fmt, stmt.pat().unwrap())));
    if let Some(ty) = stmt.ty() {
        lhs.push_token(stmt.colon_token().unwrap(), Comma);
        lhs.push(format_type(fmt, ty));
    }
    let lhs = lhs.into_frag();
    // `let pat = init else { expr };` on a single line (Style Guide), started like `builder`
    let mut single_line = None;
    match (stmt.initializer(), stmt.let_else()) {
        (Some(init), Some(let_else)) => {
            let block = let_else.block_expr().unwrap();
            let tail = single_line_block_expr(&block).map(|expr| format_expr(fmt, expr));
            let else_keyword = fmt.token(let_else.else_token().unwrap(), Word);
            let eq = fmt.token(stmt.eq_token().unwrap(), Operator);
            let init = format_head_expr(fmt, init);
            if let (Some(tail), Some(init)) = (tail.clone(), init.other.clone()) {
                let mut flat = builder.clone();
                flat.push(lhs.clone());
                flat.push(eq.clone());
                flat.push(init);
                flat.push(else_keyword.clone());
                flat.push(format_braced_block(fmt, &block, tail));
                single_line = Some(flat);
            }
            let else_block = Builder::concat(
                fmt,
                [else_keyword, format_block_expr_with_tail(fmt, block, tail)],
            );
            builder.push(format_let_else(fmt, lhs, eq, init, else_block));
        }
        (Some(init), None) => {
            builder.push(lhs);
            builder.push_token(stmt.eq_token().unwrap(), Operator);
            builder.push(fmt.same_or_indented_line_saving_lines(format_expr(fmt, init)));
        }
        (None, _) => builder.push(lhs),
    }
    let with_semicolon = |mut builder: Builder| {
        builder.push_separator(SyntaxKind::SEMICOLON);
        builder.into_frag()
    };
    // The whole statement within `single_line_let_else_max_width` (as in rustfmt)
    let single_line = single_line.map(with_semicolon).and_then(|single_line| {
        fmt.flat_within(single_line, fmt.config().single_line_let_else_max_width)
    });
    fmt.alt_opt(with_semicolon(builder), single_line)
}

/// `pat = init else {` of `let pat = init else { ... };` (Style Guide "else blocks (let-else
/// statements)"), `init` after `=`. `else {` follows the initializer on its line unless the
/// initializer ends with an indented continuation line (a broken chain, or the initializer moved to
/// the line after `=`); then it goes on its own line. It also goes there, at no cost, after an
/// initializer on a single line on the line of `=`: "If the let statement components preceding the
/// else can be formatted on a single line, but the else { does not fit on the same line, break the
/// line before the else." After a broken pattern (or type), `else {` follows only an initializer
/// spanning more lines, whose last line is then closing delimiters at the indentation of `let`
/// (`} = x` / `else {`, as in rustfmt).
fn format_let_else(
    fmt: &Formatter,
    lhs: Frag,
    eq: Frag,
    init: HeadLayouts,
    else_block: Frag,
) -> Frag {
    let (same_line, next_line) = match init.other {
        Some(other) => {
            let (same_line, next_line) = fmt.same_and_indented_line(other);
            (Some(same_line), Some(next_line))
        }
        None => (None, None),
    };
    let hanging = init.hanging.map(|h| fmt.same_or_indented_line(h));
    let flat = same_line.clone().map(|same_line| fmt.flat(same_line));
    let hanging = [hanging, next_line, flat]
        .into_iter()
        .flatten()
        .reduce(|a, b| fmt.alt_frag(a, b));
    // `else {` on its own line after a hanging initializer, shared by both left-hand sides
    let hanging_else = hanging.map(|hanging| {
        let head = HeadLayouts {
            hanging: Some(hanging),
            other: None,
        };
        format_head_followed_by(fmt, &head, else_block.clone())
    });
    // The initializer placed by `other`, then the hanging one
    let init_else = |other: Option<Frag>| {
        let other_else = other.map(|other| {
            format_head_followed_by(fmt, &HeadLayouts::other(other), else_block.clone())
        });
        fmt.alt_some(hanging_else.clone(), other_else)
            .expect("an initializer has a layout")
    };
    let flat_lhs = init_else(same_line.clone());
    let broken_lhs = init_else(same_line.map(|same_line| fmt.multi_line(same_line)));
    fmt.alt_frag(
        Builder::concat(fmt, [fmt.flat(lhs.clone()), eq.clone(), flat_lhs]),
        Builder::concat(fmt, [fmt.multi_line(lhs), eq, broken_lhs]),
    )
}

/// An expression statement; `semicolon: false` drops its `;` (see `format_stmt_list`)
fn format_expr_stmt(fmt: &Formatter, stmt: ExprStmt, semicolon: bool) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_expr_at(
        fmt,
        stmt.expr().unwrap(),
        ExprPosition::Statement,
    ));
    // Block-like expressions (`if`, ...) may stand without `;`
    if stmt.semicolon_token().is_some() {
        if semicolon {
            builder.push_separator(SyntaxKind::SEMICOLON);
        } else {
            builder.skip_separator(SyntaxKind::SEMICOLON);
        }
    }
    builder.into_frag()
}

fn format_stmt(fmt: &Formatter, stmt: Stmt) -> Frag {
    match stmt {
        Stmt::ExprStmt(stmt) => format_expr_stmt(fmt, stmt, true),
        Stmt::Item(item) => format_item(fmt, item),
        Stmt::LetStmt(stmt) => {
            format_skippable_with_attrs(fmt, &stmt, || format_let_stmt(fmt, stmt.clone()))
        }
    }
}

/// The statements of a broken block; `tail` is the already formatted tail expression, when it is
/// shared with another layout. A `return`, `break` or `continue` ending the block gets `;` with
/// `trailing_semicolon` and loses it without (as rustfmt): `break;`, `break`.
fn format_stmt_list(fmt: &Formatter, stmt_list: StmtList, tail: Option<Frag>) -> Frag {
    let delims = Delimiters::braces_of(stmt_list.syntax());
    let trailing_semicolon = fmt.config().trailing_semicolon;
    let tail = stmt_list.tail_expr().map(|expr| {
        let node = expr.syntax().clone();
        let tail = tail.unwrap_or_else(|| format_expr_at(fmt, expr, ExprPosition::Tail));
        format_block_tail(fmt, &node, tail)
    });
    let last_jump = (!trailing_semicolon)
        .then(|| trailing_jump_stmt(&stmt_list))
        .flatten();
    let lines = stmt_list
        .attrs()
        .map(|attr| format_attr(fmt, attr))
        .chain(stmt_list.statements().map(|stmt| match stmt {
            Stmt::ExprStmt(stmt) if last_jump.as_ref() == Some(&stmt) => {
                format_expr_stmt(fmt, stmt, false)
            }
            stmt => format_stmt(fmt, stmt),
        }))
        .chain(tail);
    delims.format_block(fmt, lines)
}

/// `tail`, the formatted tail expression `node` of a block, followed by `;` if it is a `return`,
/// `break` or `continue` and `trailing_semicolon` is on (as rustfmt): `break;`
pub(super) fn format_block_tail(fmt: &Formatter, node: &SyntaxNode, tail: Frag) -> Frag {
    if !(fmt.config().trailing_semicolon && is_jump_expr(node)) {
        return tail;
    }
    let mut builder = Builder::new(fmt);
    builder.push(tail);
    builder.push_generated(SEMICOLON_DOC, Comma);
    builder.into_frag()
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
    debug_assert_attrs_placed(fmt, &body);
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
