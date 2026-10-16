//! `match` and loops

use super::attr::{format_attr, format_with_attrs};
use super::common::push_label;
use super::expr::{
    ExprPosition, NestedPlainBlocks, format_added_braces, format_expr, format_expr_at,
    format_head_expr, format_head_with_block, plain_stmt_list, single_line_block_expr,
};
use super::pat::pat_layouts;
use super::stmt::{format_block_expr, format_block_expr_with_tail};
use crate::comments::has_comments;
use crate::doc::AlignGroup;
use crate::formatter::Edge::{Operator, Word};
use crate::formatter::{Builder, Formatter, Frag};
use crate::layout::head::HeadLayouts;
use crate::layout::list::Delimiters;
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

/// `'a: for pat in iterable {`; a broken head is preferably broken before `in` (Style Guide): `for pat`
/// / `    in iterable` / `{`; a broken pattern costs a split, a broken or-pattern hangs (see
/// `pat_layouts`)
pub(super) fn format_for_expr(fmt: &Formatter, expr: ForExpr) -> Frag {
    let mut builder = Builder::new(fmt);
    push_label(fmt, &mut builder, expr.label());
    let pat = pat_layouts(fmt, expr.pat().unwrap());
    let all_pat = pat.clone().all(fmt);
    let in_keyword = fmt.keyword(expr.in_token().unwrap());
    let iterable = format_head_expr(fmt, expr.iterable().unwrap());
    let head =
        |pat: Frag, iterable: Frag| Builder::concat(fmt, [pat, in_keyword.clone(), iterable]);
    let other = (iterable.other.clone())
        .zip(pat.other)
        .map(|(iterable, pat)| head(pat, iterable));
    // Broken before `in` is preferred to a hanging iterable on the line of `for`
    let mut hanging = iterable
        .hanging
        .clone()
        .map(|iterable| fmt.dispreferred(head(all_pat.clone(), iterable)));
    // A broken or-pattern, `in` on its last line
    if let (Some(pat), Some(iterable)) = (pat.hanging, iterable.other.clone()) {
        hanging = fmt.alt_some(hanging, Some(head(pat, iterable)));
    }
    // The `in` line hangs
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
    let head = format_head_expr(fmt, expr.expr().unwrap());
    let list = expr.match_arm_list().unwrap();
    let group = fmt.align_group();
    let lines = list.attrs().map(|attr| format_attr(fmt, attr)).chain(
        list.arms()
            .map(|arm| format_with_attrs(fmt, &arm, false, || format_match_arm(fmt, &arm, group))),
    );
    let arms = Delimiters::braces_of(list.syntax()).format_block(fmt, lines);
    format_head_with_block(fmt, expr.match_token().unwrap(), &head, arms)
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

/// The Style Guide's control-flow expressions, which are put in a block as a match arm body
fn is_control_flow(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::IfExpr(_) | Expr::LoopExpr(_) | Expr::WhileExpr(_) | Expr::ForExpr(_)
    )
}

/// The layouts of a match arm body: on the line of `=>` (`None` if it may not be there), and in a
/// block (`=> {` / `    body` / `}`). A body broken over more lines costs a split in both, so a body
/// that fits on one line in the block goes there, as in rustfmt; a control-flow expression is always
/// in a block (Style Guide).
fn arm_body_layouts(fmt: &Formatter, body: Expr) -> (Option<Frag>, Frag) {
    // `blocks`: the plain blocks around the expression in the source (`{ expr }`, `{ { expr } }`),
    // removed when the body stays on the line of `=>`; a plain block that is not `{ expr }` stays a
    // block (only the outer ones of nested blocks are removed)
    let (expr, blocks) = match body {
        Expr::BlockExpr(block) if plain_stmt_list(&block).is_some() => {
            let blocks = NestedPlainBlocks::of(block);
            match single_line_block_expr(&blocks.inner) {
                Some(expr) => (expr, Some(blocks)),
                None => {
                    let block = format_block_expr(fmt, blocks.inner.clone());
                    return (None, blocks.skip_outer_braces(fmt, block));
                }
            }
        }
        body => (body, None),
    };
    let control_flow = is_control_flow(&expr);
    let inner = format_expr_at(fmt, expr, ExprPosition::ArmBody);
    let inner = fmt.keep_together(inner);
    let block = match &blocks {
        Some(blocks) => {
            let block = format_block_expr_with_tail(fmt, blocks.inner.clone(), Some(inner.clone()));
            blocks.skip_outer_braces(fmt, block)
        }
        None => format_added_braces(fmt, inner.clone()),
    };
    let same_line = (!control_flow).then(|| match &blocks {
        Some(blocks) => blocks.skip_braces(fmt, inner),
        None => inner,
    });
    (same_line, block)
}

/// `pat if guard => body,` (Style Guide "Match"): the body on the line of `=>` with a comma, or in a
/// block without one (see `arm_body_layouts`). A broken left-hand side costs a split; with a guard, it
/// breaks before `if`, and the block starts on its own line: `pat` / `    if guard =>` / `{`.
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
    let (same_line, block) = arm_body_layouts(fmt, arm.expr().unwrap());

    let mut lhs = Builder::new(fmt);
    // A broken left-hand side with a guard breaks before `if`, so the guard stays on one line here,
    // and the pattern does not end on a `| C` line (Style Guide, see `pat_layouts`)
    lhs.push(if guard.is_some() {
        pat.other.expect("a pattern has a single-line layout")
    } else {
        all_pat.clone()
    });
    if let Some(guard) = &guard {
        lhs.push(fmt.flat(guard.clone()));
    }
    lhs.push(arrow.clone());
    let with_body = |mut builder: Builder, body: Frag, comma: bool| {
        builder.push(body);
        builder.align_trailing_comments(group);
        if comma {
            builder.push_separator(SyntaxKind::COMMA);
        } else {
            builder.skip_separator(SyntaxKind::COMMA);
        }
        // The comments ending the line of the arm are its trailing comments, aligned in `group`
        builder.push_comments_ending_line();
        builder.into_frag()
    };
    let mut layouts = with_body(lhs.clone(), block.clone(), false);
    if let Some(same_line) = same_line {
        layouts = fmt.alt_frag(with_body(lhs, same_line, true), layouts);
    }
    if let Some(guard) = guard {
        let mut broken = Builder::new(fmt);
        broken.push(all_pat);
        // Preferred to a broken pattern: "Avoid splitting the left-hand side" (Style Guide). The
        // block starts on its own line, an empty one stays on the line of the guard (as in rustfmt).
        let broken = if is_empty_block(&arm.expr().unwrap()) {
            broken.push(fmt.indented_line(Builder::concat(fmt, [guard, arrow, block])));
            with_body(broken, Frag::empty(), false)
        } else {
            broken.push(fmt.indented_line(Builder::concat(fmt, [guard, arrow])));
            with_body(broken, fmt.on_next_line(block), false)
        };
        layouts = fmt.alt_frag(layouts, broken);
    }
    layouts
}
