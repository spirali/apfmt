//! Closures

use super::attr::debug_assert_attrs_placed;
use super::control::match_expr_layouts;
use super::expr::{
    NestedPlainBlocks, format_added_braces, format_block_or_inline, format_expr,
    inline_and_broken_block, plain_stmt_list, single_line_block_expr,
};
use super::generics::format_for_binder;
use super::item::closure_header_layouts;
use super::ty::format_ret_type;
use crate::format::common::push_modifiers;
use crate::formatter::{Builder, Formatter, Frag};
use crate::syntax_utils::has_attrs;
use ra_ap_syntax::AstNode;
use ra_ap_syntax::ast::{BlockExpr, ClosureExpr, Expr};

/// The body of a closure, with the plain blocks around it (`{ { ... } }`, see `NestedPlainBlocks`)
enum ClosureBody {
    /// Formatted as it is: `unsafe {`, statements, a block after a return type
    Block(BlockExpr),
    /// Nested plain blocks around a block with statements: the outer braces are removed
    InnerBlock(NestedPlainBlocks),
    /// `{ expr }`, possibly nested: `expr` without braces, or in the innermost braces
    InBraces(NestedPlainBlocks, Expr),
    /// Not a block: braces are added if it needs them
    Expr(Expr),
}

fn closure_body(expr: &ClosureExpr) -> ClosureBody {
    let body = expr.body().unwrap();
    let Expr::BlockExpr(block) = body else {
        return ClosureBody::Expr(body);
    };
    if plain_stmt_list(&block).is_none() || expr.ret_type().is_some() {
        return ClosureBody::Block(block);
    }
    let blocks = NestedPlainBlocks::of(block.clone());
    match single_line_block_expr(&blocks.inner) {
        Some(inner) => ClosureBody::InBraces(blocks, inner),
        None if blocks.inner != block => ClosureBody::InnerBlock(blocks),
        None => ClosureBody::Block(block),
    }
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
    let unbraced = |inner: Frag, may_break: bool| if may_break { inner } else { fmt.flat(inner) };
    match closure_body(expr) {
        ClosureBody::Block(block) => {
            let block = format_block_or_inline(fmt, block);
            (block.clone(), Some(block))
        }
        ClosureBody::InnerBlock(blocks) => {
            let inner = format_expr(fmt, Expr::BlockExpr(blocks.inner.clone()));
            let body = blocks.skip_braces(fmt, inner);
            (body.clone(), Some(body))
        }
        ClosureBody::InBraces(blocks, inner) => {
            let may_break = may_break_unbraced(&inner);
            // Removing the braces does not break the head of a `match`: without them, the next
            // pass would not offer the layouts that the braces allow here (see `overflow_layout`)
            let (inner, without_braces) = match &inner {
                Expr::MatchExpr(match_expr) if !has_attrs(match_expr) => {
                    match_expr_layouts(fmt, match_expr)
                }
                _ => {
                    let inner = format_expr(fmt, inner);
                    let without_braces = unbraced(inner.clone(), may_break);
                    (inner, Some(without_braces))
                }
            };
            let unbraced = without_braces.map(|inner| blocks.skip_braces(fmt, inner));
            if !keep_braces {
                let braced = blocks.format_with_tail(fmt, inner);
                let all = fmt.alt_some(unbraced, Some(braced));
                return (all.expect("braces have a layout"), None);
            }
            let (inline, braced) = inline_and_broken_block(fmt, blocks.inner.clone(), inner);
            let inline = blocks.skip_outer_braces(fmt, inline);
            let braced = blocks.skip_outer_braces(fmt, braced);
            let all = fmt.alt_some(unbraced, Some(braced.clone()));
            (
                all.expect("braces have a layout"),
                Some(fmt.alt_frag(inline, braced)),
            )
        }
        ClosureBody::Expr(inner) => {
            let may_break = may_break_unbraced(&inner);
            let node = inner.syntax().clone();
            let inner = format_expr(fmt, inner);
            // Braces around a body that may break without them cost a split, as in a match arm:
            // `|x| Foo {` rather than `|x| {` / `Foo {`
            let braced = format_added_braces(fmt, &node, inner.clone());
            let braced = if may_break {
                fmt.dispreferred(braced)
            } else {
                braced
            };
            let all = fmt.alt_frag(unbraced(inner, may_break), braced);
            (all.clone(), Some(all))
        }
    }
}
