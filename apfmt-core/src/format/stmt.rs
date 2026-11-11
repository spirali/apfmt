//! Statements and blocks

use super::attr::{debug_assert_attrs_placed, format_attr, format_skippable_with_attrs};
use super::expr::{
    ExprPosition, format_braced_block, format_expr, format_expr_at, format_head_expr,
    format_head_followed_by, format_rhs, push_head_after_comments, single_line_block_expr,
};
use super::format_unsupported;
use super::item::format_item;
use super::pat::format_pat;
use super::ty::format_type_annotation;
use crate::comments::comments_break_line_after;
use crate::doc::SEMICOLON_DOC;
use crate::format::common::{push_label, push_modifiers};
use crate::formatter::Edge::{Comma, Operator, Word};
use crate::formatter::{Builder, Formatter, Frag, NextLine};
use crate::layout::head::HeadLayouts;
use crate::layout::list::Delimiters;
use crate::syntax_utils::{is_jump_expr, trailing_jump_stmt};
use ra_ap_syntax::ast::{BlockExpr, ExprStmt, HasAttrs, LetStmt, Stmt, StmtList};
use ra_ap_syntax::{AstNode, SyntaxKind, SyntaxNode};

fn format_let_stmt(fmt: &Formatter, stmt: LetStmt) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_keyword(stmt.let_token().unwrap());
    // `pat: Type`; the pattern is broken only where it does not fit on one line, whatever the
    // initializer could gain (Style Guide: the left-hand side is split last)
    let after_lhs = if stmt.initializer().is_some() {
        " ="
    } else {
        ";"
    };
    let after_pat = if stmt.ty().is_some() { ":" } else { after_lhs };
    let pat = fmt.keep_together(format_pat(fmt, stmt.pat().unwrap()));
    let mut lhs = Builder::new(fmt);
    lhs.push(fmt.flat_where_fits(pat, after_pat.len() as u32));
    if let Some(ty) = stmt.ty() {
        let colon = stmt.colon_token().unwrap();
        lhs.push(format_type_annotation(
            fmt,
            colon,
            ty,
            after_lhs.len() as u32,
        ));
    }
    let lhs = lhs.into_frag();
    // `let pat = init else { expr };` on a single line (Style Guide)
    let mut single_line = None;
    match (stmt.initializer(), stmt.let_else()) {
        (Some(init), Some(let_else)) => {
            let block = let_else.block_expr().unwrap();
            let eq_token = stmt.eq_token().unwrap();
            // Comments ending the line of `=` put the initializer on an indented line: `let x = //
            // c` / `    init`
            let comment_after_eq = comments_break_line_after(&eq_token);
            let tail = (!comment_after_eq)
                .then(|| single_line_block_expr(&block).map(|expr| format_expr(fmt, expr)))
                .flatten();
            let else_keyword = fmt.token(let_else.else_token().unwrap(), Word);
            let eq = fmt.token(eq_token, Operator);
            // `else {` follows only closing delimiters (Style Guide), unlike `{` of a control flow
            // head after `)?`
            let ends_with_try = init
                .syntax()
                .last_token()
                .is_some_and(|token| token.kind() == SyntaxKind::QUESTION);
            let mut init = format_head_expr(fmt, init);
            if ends_with_try {
                init = init.other_hanging_when_broken(fmt);
            }
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
            if comment_after_eq {
                builder.push(lhs);
                builder.push(eq);
                push_head_after_comments(fmt, &mut builder, init, else_block);
            } else {
                builder.push(format_let_else(fmt, lhs, eq, init, else_block));
            }
        }
        (Some(init), None) => {
            builder.push(lhs);
            builder.push_token(stmt.eq_token().unwrap(), Operator);
            builder.push(format_rhs(fmt, init));
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

/// `pat = init else {`: `else {` follows the initializer unless it ends with a continuation line,
/// or it does not fit after a flat one (Style Guide); after a broken pattern only after `} = x`.
fn format_let_else(
    fmt: &Formatter,
    lhs: Frag,
    eq: Frag,
    init: HeadLayouts,
    else_block: Frag,
) -> Frag {
    let (same_line, next_line) = match init.other {
        Some(other) => {
            let (same_line, next_line) =
                fmt.same_and_indented_line(other, NextLine::SavingTwoLines);
            (Some(same_line), Some(next_line))
        }
        None => (None, None),
    };
    let hanging = (init.hanging).map(|h| fmt.same_or_indented_line(h, NextLine::SavingTwoLines));
    let flat = same_line.clone().map(|same_line| fmt.flat(same_line));
    let hanging = [hanging, next_line, flat]
        .into_iter()
        .flatten()
        .reduce(|a, b| fmt.alt_frag(a, b));
    // Shared by both left-hand sides
    let hanging_else = hanging.map(|hanging| {
        let head = HeadLayouts {
            hanging: Some(hanging),
            other: None,
        };
        format_head_followed_by(fmt, &head, else_block.clone())
    });
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

/// `semicolon: false` drops its `;`
fn format_expr_stmt(fmt: &Formatter, stmt: ExprStmt, semicolon: bool) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_expr_at(
        fmt,
        stmt.expr().unwrap(),
        ExprPosition::Statement,
    ));
    // Block-like expressions may stand without `;`
    if stmt.semicolon_token().is_some() {
        builder.push_or_skip_separator(SyntaxKind::SEMICOLON, semicolon);
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

/// `tail` is the formatted tail expression, when shared. With `normalize_jump`, a final jump gets
/// `;` with `trailing_semicolon` and loses it without (as rustfmt).
fn format_stmt_list(
    fmt: &Formatter,
    stmt_list: StmtList,
    tail: Option<Frag>,
    normalize_jump: bool,
) -> Frag {
    let delims = Delimiters::braces_of(stmt_list.syntax());
    let trailing_semicolon = fmt.config().trailing_semicolon;
    let tail = stmt_list.tail_expr().map(|expr| {
        let node = expr.syntax().clone();
        let tail = tail.unwrap_or_else(|| format_expr_at(fmt, expr, ExprPosition::Tail));
        if normalize_jump {
            format_block_tail(fmt, &node, tail)
        } else {
            tail
        }
    });
    let last_jump = (normalize_jump && !trailing_semicolon)
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

/// Followed by `;` if it is a jump and `trailing_semicolon` is on (as rustfmt)
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

/// `'a: {`, `unsafe {`, `async move {`, `const {`, `gen {`
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
    builder.push(format_stmt_list(fmt, body.stmt_list().unwrap(), tail, true));
    builder.into_frag()
}

/// Broken; a final jump keeps its `;` as in the source, since it would end up in an expression
/// where the macro is used (as rustfmt)
pub(super) fn format_transcriber_block(fmt: &Formatter, body: BlockExpr) -> Frag {
    format_stmt_list(fmt, body.stmt_list().unwrap(), None, false)
}

/// `'a: `, `unsafe `, `async move `
pub(super) fn push_block_prefix(fmt: &Formatter, builder: &mut Builder, body: &BlockExpr) {
    push_label(fmt, builder, body.label());
    push_modifiers(builder, body.syntax());
}
