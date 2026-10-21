//! Closures

use super::attr::debug_assert_attrs_placed;
use super::expr::{
    NestedPlainBlocks, format_added_braces, format_block_or_inline, format_expr,
    inline_and_broken_block, plain_stmt_list, single_line_block_expr,
};
use super::generics::format_for_binder;
use super::item::closure_params_layouts;
use super::stmt::format_block_expr_with_tail;
use super::ty::format_ret_type;
use crate::format::common::push_modifiers;
use crate::formatter::{Builder, Formatter, Frag};
use crate::syntax_utils::has_attrs;
use ra_ap_syntax::AstNode;
use ra_ap_syntax::ast::{ClosureExpr, Expr};

/// The body of a closure that may be written without braces: the only expression of a plain block
/// `{ expr }` (no statements, comments, label or modifiers) of a closure without a return type, or a
/// body that is not a plain block. Of nested plain blocks, `{ { g(); x } }`, the innermost one is the
/// body when it is not `{ expr }`.
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

/// Whether a closure body without braces may span more lines: a block-like expression (`|x| unsafe {`
/// / `...` / `}`, `|x| match x {`, `|| Foo {`); other expressions get braces then (Style Guide: "Add
/// the `{}` when ... the body expression is a control-flow expression that spans multiple lines";
/// `match` is not one of the Style Guide's control-flow expressions, loops are)
fn is_block_like_expr(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::BlockExpr(_) | Expr::MatchExpr(_) | Expr::RecordExpr(_)
    )
}

pub(super) fn format_closure_expr(fmt: &Formatter, expr: ClosureExpr) -> Frag {
    closure_layouts(fmt, expr).all
}

/// The layouts of a closure: all of them, and those that may overflow as the last argument of a call,
/// with the parameters on one line (the Style Guide combines "a multi-line closure with an explicit
/// block"; rustfmt does not combine a closure with broken parameters either)
pub(super) struct ClosureLayouts {
    pub all: Frag,
    pub overflow: Option<Frag>,
}

/// `move |a, b| body` or `for<'a> |a: &'a T| body` (Style Guide "Closures"), see `closure_body_layouts`. Parameters that do not fit
/// on one line are aligned after `|` (see `closure_params_layouts`); a body in braces then keeps them,
/// as in rustfmt.
pub(super) fn closure_layouts(fmt: &Formatter, expr: ClosureExpr) -> ClosureLayouts {
    debug_assert_attrs_placed(fmt, &expr);
    let mut prefix = Builder::new(fmt);
    if let Some(binder) = expr.for_binder() {
        prefix.push(format_for_binder(fmt, binder));
    }
    push_modifiers(&mut prefix, expr.syntax());
    let (flat_params, broken_params) = closure_params_layouts(fmt, expr.param_list().unwrap());
    let ret_type = expr
        .ret_type()
        .map(|ret_type| format_ret_type(fmt, ret_type));
    let (body, braced_body) = closure_body_layouts(fmt, &expr, broken_params.is_some());
    let closure = |params: Frag, body: Frag| {
        let mut builder = prefix.clone();
        builder.push(fmt.followed_by_space(params));
        if let Some(ret_type) = &ret_type {
            builder.push(ret_type.clone());
        }
        builder.push(body);
        builder.into_frag()
    };
    let flat = flat_params.map(|params| closure(params, body));
    // Broken parameters cost a split: only when they do not fit otherwise
    let broken = broken_params
        .zip(braced_body)
        .map(|(params, body)| closure(fmt.dispreferred(params), body));
    let all = fmt.alt_some(flat.clone(), broken);
    ClosureLayouts {
        all: all.expect("parameters have a layout"),
        overflow: flat,
    }
}

/// The layouts of a closure body: all of them, and, with `keep_braces`, those keeping the braces of
/// the source (after broken parameters, as in rustfmt). Without braces if possible, i.e. on one
/// line, or block-like (see `is_block_like_expr`); otherwise in braces, which are added if the
/// source has none: `|a| {` / `    body` / `}`.
fn closure_body_layouts(
    fmt: &Formatter,
    expr: &ClosureExpr,
    keep_braces: bool,
) -> (Frag, Option<Frag>) {
    let body = expr.body().unwrap();
    // A body with attributes (`|x| #[allow(a)] {`) is copied unchanged with them, see `format_expr`
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
    let block_like = is_block_like_expr(&inner);
    let inner_node = inner.syntax().clone();
    let inner_frag = format_expr(fmt, inner);
    let unbraced = if block_like {
        inner_frag.clone()
    } else {
        fmt.flat(inner_frag.clone())
    };
    match body {
        // The braces of the source are removed, or only the outer ones of nested blocks
        Expr::BlockExpr(block) => {
            let blocks = NestedPlainBlocks::of(block);
            let unbraced = blocks.skip_braces(fmt, unbraced);
            // The innermost block is the body, `{ { g(); x } }`: its braces are kept
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
            // Braces added around a block-like body, which needs none, cost a split, as in a match
            // arm (see `arm_body_layouts`): `|x| Foo {` / `...` / `}` rather than `|x| {` / `Foo {`
            let braced = format_added_braces(fmt, &inner_node, inner_frag);
            let braced = if block_like {
                fmt.dispreferred(braced)
            } else {
                braced
            };
            let all = fmt.alt_frag(unbraced, braced);
            (all.clone(), Some(all))
        }
    }
}
