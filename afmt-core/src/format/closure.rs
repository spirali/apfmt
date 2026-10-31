//! Closures

use super::attr::debug_assert_attrs_placed;
use super::expr::{
    NestedPlainBlocks, format_added_braces, format_block_or_inline, format_expr,
    inline_and_broken_block, plain_stmt_list, single_line_block_expr,
};
use super::generics::format_for_binder;
use super::item::closure_header_layouts;
use super::stmt::format_block_expr_with_tail;
use super::ty::format_ret_type;
use crate::format::common::push_modifiers;
use crate::formatter::{Builder, Formatter, Frag};
use crate::syntax_utils::has_attrs;
use ra_ap_syntax::AstNode;
use ra_ap_syntax::ast::{ClosureExpr, Expr};

/// The body without braces: the expression of `{ expr }` (no statements, comments, label or return
/// type), or a body that is not a plain block; of nested blocks, the innermost
fn unbraced_closure_body(expr: &ClosureExpr) -> Option<Expr> {
    let body = expr.body()?;
    let Expr::BlockExpr(block) = &body else {
        return Some(body);
    };
    if plain_stmt_list(block).is_none() {
        return Some(body);
    }
    if expr.ret_type().is_some() {
        return None;
    }
    let inner = NestedPlainBlocks::of(block.clone()).inner;
    single_line_block_expr(&inner).or_else(|| (inner != *block).then_some(Expr::BlockExpr(inner)))
}

/// A body without braces that may span more lines: `|x| unsafe {`, `|x| match x {`, `|| Foo {`;
/// others get braces (Style Guide)
fn may_break_unbraced(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::BlockExpr(_) | Expr::MatchExpr(_) | Expr::RecordExpr(_)
    )
}

pub(super) fn format_closure_expr(fmt: &Formatter, expr: ClosureExpr) -> Frag {
    closure_layouts(fmt, expr).all
}

/// All layouts, and those that may overflow as the last call argument, with flat parameters (as
/// the Style Guide and rustfmt)
pub(super) struct ClosureLayouts {
    pub all: Frag,
    pub overflow: Option<Frag>,
}

/// `move |a, b| body`: parameters that do not fit are aligned after `|`; a body in braces keeps
/// them
pub(super) fn closure_layouts(fmt: &Formatter, expr: ClosureExpr) -> ClosureLayouts {
    debug_assert_attrs_placed(fmt, &expr);
    let mut prefix = Builder::new(fmt);
    if let Some(binder) = expr.for_binder() {
        prefix.push(format_for_binder(fmt, binder));
    }
    push_modifiers(&mut prefix, expr.syntax());
    // The return type is broken only when nothing else fits (rustfmt never breaks it)
    let ret_type = expr.ret_type().map(|ret_type| {
        let ret_type = format_ret_type(fmt, ret_type);
        match fmt.flat_if_possible(ret_type.clone()) {
            Some(flat) => fmt.alt_frag(flat, fmt.dispreferred(ret_type)),
            None => ret_type,
        }
    });
    let (flat_header, broken_header) =
        closure_header_layouts(fmt, expr.param_list().unwrap(), ret_type);
    let (body, braced_body) = closure_body_layouts(fmt, &expr, broken_header.is_some());
    let closure = |header: Frag, body: Frag| {
        let mut builder = prefix.clone();
        builder.push(fmt.followed_by_space(header));
        builder.push(body);
        builder.into_frag()
    };
    let flat = flat_header.map(|header| closure(header, body));
    let broken = broken_header
        .zip(braced_body)
        .map(|(header, body)| closure(fmt.dispreferred(header), body));
    let all = fmt.alt_some(flat.clone(), broken);
    ClosureLayouts {
        all: all.expect("parameters have a layout"),
        overflow: flat,
    }
}

/// With `keep_braces`, also those keeping the source braces. Without braces if flat or
/// `may_break_unbraced`, otherwise in braces: `|a| {` / `    body` / `}`.
fn closure_body_layouts(
    fmt: &Formatter,
    expr: &ClosureExpr,
    keep_braces: bool,
) -> (Frag, Option<Frag>) {
    let body = expr.body().unwrap();
    // Copied unchanged with its attributes, see `format_expr`
    if has_attrs(&body) {
        let body = format_expr(fmt, body);
        return (body.clone(), Some(body));
    }
    let inner = match unbraced_closure_body(expr) {
        // Braces required, or the body is a block itself: `unsafe {`
        Some(inner) if inner.syntax() != body.syntax() || !matches!(inner, Expr::BlockExpr(_)) => {
            inner
        }
        _ => {
            let Expr::BlockExpr(block) = body else {
                unreachable!("checked by `unbraced_closure_body` and above");
            };
            let block = format_block_or_inline(fmt, block);
            return (block.clone(), Some(block));
        }
    };
    let may_break = may_break_unbraced(&inner);
    let inner_node = inner.syntax().clone();
    let inner_frag = format_expr(fmt, inner);
    let unbraced = if may_break {
        inner_frag.clone()
    } else {
        fmt.flat(inner_frag.clone())
    };
    match body {
        Expr::BlockExpr(block) => {
            let blocks = NestedPlainBlocks::of(block);
            let unbraced = blocks.skip_braces(fmt, unbraced);
            if blocks.inner.syntax() == &inner_node {
                return (unbraced.clone(), Some(unbraced));
            }
            if !keep_braces {
                let braced =
                    format_block_expr_with_tail(fmt, blocks.inner.clone(), Some(inner_frag));
                let braced = blocks.skip_outer_braces(fmt, braced);
                return (fmt.alt_frag(unbraced, braced), None);
            }
            let (inline, braced) = inline_and_broken_block(fmt, blocks.inner.clone(), inner_frag);
            let inline = blocks.skip_outer_braces(fmt, inline);
            let braced = blocks.skip_outer_braces(fmt, braced);
            let all = fmt.alt_frag(unbraced, braced.clone());
            (all, Some(fmt.alt_frag(inline, braced)))
        }
        _ => {
            // Braces around a body that may break without them cost a split, as in a match arm:
            // `|x| Foo {` rather than `|x| {` / `Foo {`
            let braced = format_added_braces(fmt, &inner_node, inner_frag);
            let braced = if may_break {
                fmt.dispreferred(braced)
            } else {
                braced
            };
            let all = fmt.alt_frag(unbraced, braced);
            (all.clone(), Some(all))
        }
    }
}
