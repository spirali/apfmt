//! `match` and loops

use super::attr::{format_attr, format_with_attrs};
use super::chain::operand_chain;
use super::common::push_label;
use super::expr::{
    ExprPosition, NestedPlainBlocks, format_added_braces, format_expr, format_expr_at,
    format_head_expr, format_head_with_block, plain_stmt_list, single_line_block_expr,
};
use super::expr_list::{ExprItem, format_expr_item};
use super::pat::pat_layouts;
use super::stmt::{format_block_expr, format_block_expr_with_tail};
use crate::comments::{comments_break_line_after, has_comments};
use crate::doc::AlignGroup;
use crate::formatter::Edge::{Operator, Word};
use crate::formatter::{Builder, Formatter, Frag};
use crate::layout::head::{HeadLayouts, pattern_head};
use crate::layout::list::{Delimiters, Overflow};
use crate::syntax_utils::has_attrs;
use ra_ap_syntax::ast::{
    Expr, ForExpr, HasAttrs, HasLoopBody, LoopExpr, MatchArm, MatchExpr, WhileExpr,
};
use ra_ap_syntax::{AstNode, SyntaxKind};

/// `'a: loop {`
pub(super) fn format_loop_expr(fmt: &Formatter, expr: LoopExpr) -> Frag {
    let mut builder = Builder::new(fmt);
    push_label(fmt, &mut builder, expr.label());
    builder.push_token(expr.loop_token().unwrap(), Word);
    builder.push(format_block_expr(fmt, expr.loop_body().unwrap()));
    builder.into_frag()
}

/// `'a: while cond {`, the head as in `if`
pub(super) fn format_while_expr(fmt: &Formatter, expr: WhileExpr) -> Frag {
    let mut builder = Builder::new(fmt);
    push_label(fmt, &mut builder, expr.label());
    let head = format_head_expr(fmt, expr.condition().unwrap());
    let body = format_block_expr(fmt, expr.loop_body().unwrap());
    builder.push(format_head_with_block(
        fmt,
        expr.while_token().unwrap(),
        &head,
        body,
    ));
    builder.into_frag()
}

/// `'a: for pat in iterable {`; preferably broken before `in` (Style Guide): `for pat` / `    in
/// iterable` / `{`
pub(super) fn format_for_expr(fmt: &Formatter, expr: ForExpr) -> Frag {
    let mut builder = Builder::new(fmt);
    push_label(fmt, &mut builder, expr.label());
    let pat = pat_layouts(fmt, expr.pat().unwrap());
    let all_pat = pat.clone().all(fmt);
    let in_token = expr.in_token().unwrap();
    let comment_after_in = comments_break_line_after(&in_token);
    let in_keyword = fmt.keyword(in_token);
    let mut iterable = format_head_expr(fmt, expr.iterable().unwrap());
    if comment_after_in {
        iterable = iterable.into_hanging(fmt);
    }
    let head =
        |pat: Frag, iterable: Frag| Builder::concat(fmt, [pat, in_keyword.clone(), iterable]);
    // Broken before `in` is preferred to an iterable broken on the line of `for` (Style Guide); not
    // after comments ending the line of `in`
    let on_line = HeadLayouts {
        hanging: if comment_after_in {
            iterable.hanging.clone()
        } else {
            iterable.hanging.clone().map(|i| fmt.dispreferred(i))
        },
        other: iterable.other.clone().map(|i| fmt.keep_together(i)),
    };
    let HeadLayouts { hanging, other } = pattern_head(fmt, &pat, &all_pat, &on_line, head);
    let in_line = Builder::concat(
        fmt,
        [in_keyword.clone(), fmt.keep_together(iterable.all(fmt))],
    );
    let broken = Builder::concat(fmt, [all_pat, fmt.indented_line(in_line)]);
    let head = HeadLayouts {
        hanging: fmt.alt_some(hanging, Some(broken)),
        other,
    };
    let body = format_block_expr(fmt, expr.loop_body().unwrap());
    builder.push(format_head_with_block(
        fmt,
        expr.for_token().unwrap(),
        &head,
        body,
    ));
    builder.into_frag()
}

/// `match scrutinee {`, the head as in `if`, then the arms, each on its own line (Style Guide "Match")
pub(super) fn format_match_expr(fmt: &Formatter, expr: MatchExpr) -> Frag {
    let (head, arms) = match_expr_parts(fmt, &expr);
    format_head_with_block(fmt, expr.match_token().unwrap(), &head, arms)
}

fn match_expr_parts(fmt: &Formatter, expr: &MatchExpr) -> (HeadLayouts, Frag) {
    let head = format_head_expr(fmt, expr.expr().unwrap());
    let list = expr.match_arm_list().unwrap();
    let group = fmt.align_group();
    let lines = list.attrs().map(|attr| format_attr(fmt, attr)).chain(
        list.arms()
            .map(|arm| format_with_attrs(fmt, &arm, false, || format_match_arm(fmt, &arm, group))),
    );
    let arms = Delimiters::braces_of(list.syntax()).format_item_block(fmt, lines);
    (head, arms)
}

/// `{}`, without comments
fn is_empty_block(expr: &Expr) -> bool {
    let Expr::BlockExpr(block) = expr else {
        return false;
    };
    plain_stmt_list(block).is_some_and(|list| {
        list.statements().next().is_none()
            && list.tail_expr().is_none()
            && !has_comments(list.syntax())
    })
}

/// Put in a block as a match arm body
fn is_control_flow(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::IfExpr(_) | Expr::LoopExpr(_) | Expr::WhileExpr(_) | Expr::ForExpr(_)
    )
}

struct ArmBodyLayouts {
    /// `None` if it may not be there
    same_line: Option<Frag>,
    block: Frag,
}

/// On the line of `=>`, and in a block; a broken body costs a split in both, and one more in added
/// braces. Control flow is always in a block, a single macro call keeps its braces (Style Guide).
fn arm_body_layouts(fmt: &Formatter, body: Expr) -> ArmBodyLayouts {
    // The plain blocks around the expression (`{ expr }`), removed when the body stays on the line
    // of `=>`
    let (expr, blocks) = match body {
        Expr::BlockExpr(block) if plain_stmt_list(&block).is_some() => {
            let blocks = NestedPlainBlocks::of(block);
            match single_line_block_expr(&blocks.inner) {
                Some(expr) if !matches!(expr, Expr::MacroExpr(_)) => (expr, Some(blocks)),
                _ => {
                    let block = format_block_expr(fmt, blocks.inner.clone());
                    return ArmBodyLayouts {
                        same_line: None,
                        block: blocks.skip_outer_braces(fmt, block),
                    };
                }
            }
        }
        body => (body, None),
    };
    let control_flow = is_control_flow(&expr);
    let node = expr.syntax().clone();
    let (inner, same_line_inner) = match &expr {
        // A `match` on the line of `=>` with its head on one line
        Expr::MatchExpr(match_expr) if !has_attrs(match_expr) => {
            let (head, arms) = match_expr_parts(fmt, match_expr);
            let keyword = match_expr.match_token().unwrap();
            let flat_head = head.other.clone().map(|other| {
                let mut builder = Builder::new(fmt);
                builder.push_keyword(keyword.clone());
                builder.push(fmt.flat(other));
                builder.push(arms.clone());
                fmt.keep_together(builder.into_frag())
            });
            let inner = format_head_with_block(fmt, keyword, &head, arms);
            (fmt.keep_together(inner), flat_head)
        }
        // `if` here is in a block: it gets the layouts of a statement
        Expr::BlockExpr(_) | Expr::IfExpr(_) => {
            let inner = fmt.keep_together(format_expr_at(fmt, expr, ExprPosition::ArmBody));
            (inner.clone(), Some(inner))
        }
        // A chain also broken before its elements, as in rustfmt; the Style Guide does not say
        _ if operand_chain(&expr).is_some() => {
            let inner = fmt.keep_together(format_expr(fmt, expr));
            (inner.clone(), Some(inner))
        }
        _ => {
            let ExprItem {
                frag: inner,
                overflow,
                ..
            } = format_expr_item(fmt, expr);
            // A combinable expression overflows in all its layouts, a closure in some, without the
            // single line
            let same_line = match overflow {
                Overflow::With(_) => Some(inner.clone()),
                overflow => fmt.alt_some(
                    fmt.flat_if_possible(inner.clone()),
                    overflow.layouts().cloned(),
                ),
            };
            (
                fmt.keep_together(inner),
                same_line.map(|same_line| fmt.keep_together(same_line)),
            )
        }
    };
    let block = match &blocks {
        Some(blocks) => {
            let block = format_block_expr_with_tail(fmt, blocks.inner.clone(), Some(inner.clone()));
            blocks.skip_outer_braces(fmt, block)
        }
        None => format_added_braces(fmt, &node, fmt.keep_together(inner)),
    };
    let same_line = same_line_inner
        .filter(|_| !control_flow)
        .map(|inner| match &blocks {
            Some(blocks) => blocks.skip_braces(fmt, inner),
            None => inner,
        });
    ArmBodyLayouts { same_line, block }
}

/// `pat if guard => body,`: the body after `=>` with a comma, or in a block without one (unless
/// `match_block_trailing_comma`); with a guard, broken before `if`
fn format_match_arm(fmt: &Formatter, arm: &MatchArm, group: Option<AlignGroup>) -> Frag {
    let pat = pat_layouts(fmt, arm.pat().unwrap());
    let all_pat = pat.clone().all(fmt);
    let guard = arm.guard().map(|guard| {
        let condition = format_expr(fmt, guard.condition().unwrap());
        Builder::concat(
            fmt,
            [
                fmt.keyword(guard.if_token().unwrap()),
                fmt.keep_together(condition),
            ],
        )
    });
    let arrow = fmt.token(arm.fat_arrow_token().unwrap(), Operator);
    let ArmBodyLayouts { same_line, block } = arm_body_layouts(fmt, arm.expr().unwrap());

    let mut lhs = Builder::new(fmt);
    // The guard stays on one line, and the pattern does not end on a `| C` line (Style Guide)
    lhs.push(if guard.is_some() {
        pat.other.expect("a pattern has a single-line layout")
    } else {
        all_pat.clone()
    });
    if let Some(guard) = &guard {
        lhs.push(fmt.flat(guard.clone()));
    }
    lhs.push(arrow.clone());
    // Broken only if it does not fit with room for ` {` (Style Guide: "If using a block form on the
    // right-hand side ... makes it possible to avoid breaking on the left-hand side, do that")
    let lhs = fmt.flat_where_fits(lhs.into_frag(), " {".len() as u32);
    let with_body = |lhs: Frag, body: Frag, comma: bool| {
        let mut builder = Builder::new(fmt);
        builder.push(lhs);
        builder.push(body);
        builder.align_trailing_comments(group);
        builder.push_or_skip_separator(SyntaxKind::COMMA, comma);
        // The comments ending the line of the arm are its trailing comments, aligned in `group`
        builder.push_comments_ending_line();
        builder.into_frag()
    };
    let block_comma = fmt.config().match_block_trailing_comma;
    let mut layouts = with_body(lhs.clone(), block.clone(), block_comma);
    if let Some(same_line) = same_line {
        layouts = fmt.alt_frag(with_body(lhs, same_line, true), layouts);
    }
    if let Some(guard) = guard {
        let mut broken = Builder::new(fmt);
        broken.push(all_pat);
        // "Avoid splitting the left-hand side" (Style Guide); an empty block stays on the line of
        // the guard
        let broken = if is_empty_block(&arm.expr().unwrap()) {
            broken.push(fmt.indented_line(Builder::concat(fmt, [guard, arrow, block])));
            with_body(broken.into_frag(), Frag::empty(), block_comma)
        } else {
            broken.push(fmt.indented_line(Builder::concat(fmt, [guard, arrow])));
            with_body(broken.into_frag(), fmt.on_next_line(block), block_comma)
        };
        layouts = fmt.alt_frag(layouts, broken);
    }
    layouts
}
