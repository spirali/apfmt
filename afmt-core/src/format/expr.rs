//! Expressions

use super::pat::format_pat;
use super::path::format_path;
use super::stmt::{format_block_expr, format_block_expr_with_tail};
use crate::format::common::{format_braced, format_parenthesized};
use crate::formatter::Edge::{Operator, Tight, Word};
use crate::formatter::{Builder, Formatter, Frag};
use crate::layout::chain::infix_chain;
use crate::layout::list::{
    Delimiters, FlatLimit, ListItem, ListLayout, delimited_list, list_items_cast,
};
use ra_ap_syntax::ast::{
    ArgList, BinExpr, BlockExpr, CallExpr, ElseBranch, Expr, HasArgList, IfExpr, LetExpr, Literal,
    ParenExpr, PathExpr, PrefixExpr,
};
use ra_ap_syntax::{AstNode, SyntaxKind, SyntaxToken};

fn format_path_expr(fmt: &Formatter, path_expr: PathExpr) -> Frag {
    format_path(fmt, path_expr.path().unwrap())
}

/// Whether a single argument can overflow, i.e. start on the line of `(` and end on the line of `)`:
/// `f(if a {\n ...\n})`. Follows the combinable expressions of the style guide.
// TODO: closures, `match`, loops, struct literals, arrays, tuples, method calls and macros once they are supported
fn is_combinable_expr(expr: &Expr) -> bool {
    match expr {
        Expr::CallExpr(_) | Expr::IfExpr(_) | Expr::BlockExpr(_) => true,
        // `f(!g(\n...\n))`
        Expr::PrefixExpr(expr) => expr.expr().is_some_and(|e| is_combinable_expr(&e)),
        _ => false,
    }
}

/// Simple expressions in the sense of rustfmt's `is_simple_expr`: literals, plain names and unary
/// operators applied to them
fn is_simple_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Literal(_) => true,
        Expr::PathExpr(expr) => expr.path().is_some_and(|path| {
            path.qualifier().is_none()
                && path
                    .segment()
                    .is_some_and(|s| s.type_anchor().is_none() && s.coloncolon_token().is_none())
        }),
        Expr::PrefixExpr(expr) => expr.expr().is_some_and(|e| is_simple_expr(&e)),
        _ => false,
    }
}

/// Whether a broken list of the expressions `exprs` (formatted as `items`) is filled on lines instead
/// of one item per line: all items are simple and at most `short_array_element_width_threshold` wide,
/// as in rustfmt. This is an explicit exception to the Style Guide. For call arguments now, arrays
/// and `vec![]` later.
fn is_fillable_list(
    fmt: &Formatter,
    mut exprs: impl Iterator<Item = Expr>,
    items: &[ListItem],
) -> bool {
    let threshold = fmt.config().short_array_element_width_threshold as usize;
    threshold > 0
        && exprs.all(|e| is_simple_expr(&e))
        && items.iter().all(|i| {
            fmt.simple_length_frag(&i.frag)
                .is_some_and(|w| w <= threshold)
        })
}

fn format_arg_list(fmt: &Formatter, arg_list: ArgList) -> Frag {
    let args = list_items_cast(arg_list.syntax(), |e| format_expr(fmt, e));
    let single = (args.len() == 1).then(|| arg_list.args().next()).flatten();
    let hug = single.as_ref().is_some_and(is_combinable_expr);
    // A single argument that cannot overflow is not limited by `fn_call_width`. One that can
    // (e.g. a call) is: when it does not fit, it is broken instead, like rustfmt does.
    let flat = if single.is_some() && !hug {
        FlatLimit::MaxWidth
    } else {
        FlatLimit::Within(fmt.config().fn_call_width)
    };
    let layout = ListLayout::tall(flat)
        .fill_if(is_fillable_list(fmt, arg_list.args(), &args))
        .hug_if(hug);
    delimited_list(
        fmt,
        &Delimiters::parens_of(arg_list.syntax()),
        &args,
        layout,
    )
}

fn format_call_expr(fmt: &Formatter, call: CallExpr) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_expr(fmt, call.expr().unwrap()));
    builder.push(format_arg_list(fmt, call.arg_list().unwrap()));
    builder.into_frag()
}

fn format_literal(fmt: &Formatter, literal: Literal) -> Frag {
    fmt.token(literal.token(), Word)
}

/// Splits a chain of the same binary operator, e.g. `a && b && c`, into the first operand
/// (the return value) and the (operator, operand) pairs that follow it
fn collect_bin_chain(expr: BinExpr, rest: &mut Vec<(SyntaxToken, Expr)>) -> Expr {
    let lhs = expr.lhs().unwrap();
    let first = match &lhs {
        Expr::BinExpr(inner) if inner.op_kind() == expr.op_kind() => {
            collect_bin_chain(inner.clone(), rest)
        }
        _ => lhs,
    };
    rest.push((expr.op_token().unwrap(), expr.rhs().unwrap()));
    first
}

fn format_bin_expr(fmt: &Formatter, expr: BinExpr) -> Frag {
    let mut rest = Vec::new();
    let first = format_expr(fmt, collect_bin_chain(expr, &mut rest));
    let rest = rest
        .into_iter()
        .map(|(op, e)| (fmt.token(op, Operator), format_expr(fmt, e)))
        .collect::<Vec<_>>();
    infix_chain(fmt, first, &rest)
}

fn format_prefix_expr(fmt: &Formatter, expr: PrefixExpr) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_token(expr.op_token().unwrap(), Tight);
    builder.push(format_expr(fmt, expr.expr().unwrap()));
    builder.into_frag()
}

fn format_paren_expr(fmt: &Formatter, expr: ParenExpr) -> Frag {
    let inner = format_expr(fmt, expr.expr().unwrap());
    format_parenthesized(
        fmt,
        expr.l_paren_token().unwrap(),
        inner,
        expr.r_paren_token().unwrap(),
    )
}

/// `let pat = expr` in a condition
fn format_let_expr(fmt: &Formatter, expr: LetExpr) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_keyword(expr.let_token().unwrap());
    builder.push(format_pat(fmt, expr.pat().unwrap()));
    builder.push_token(expr.eq_token().unwrap(), Operator);
    builder.push(format_expr(fmt, expr.expr().unwrap()));
    builder.into_frag()
}

/// Whether a multi-line layout of the expression ends with an indented continuation line, e.g. a broken
/// `a && b`; in the head of a control-flow expression, `{` then has to go on its own line
/// (see `format_head_with_block`)
fn is_hanging_expr(expr: &Expr) -> bool {
    match expr {
        Expr::BinExpr(_) => true,
        Expr::LetExpr(expr) => expr.expr().is_some_and(|e| is_hanging_expr(&e)),
        Expr::PrefixExpr(expr) => expr.expr().is_some_and(|e| is_hanging_expr(&e)),
        _ => false,
    }
}

/// `keyword head {`, or `keyword head` + `{` on the next line when the head is broken with a hanging
/// indent (`hanging`, see `is_hanging_expr`), so `{` is not mistaken for a part of the head. The head of
/// a control-flow expression: `if cond`, `while cond`, `match expr`, `for pat in expr`.
fn format_head_with_block(
    fmt: &Formatter,
    keyword: SyntaxToken,
    head: Frag,
    hanging: bool,
    block: Frag,
) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_keyword(keyword);

    let mut same_line = builder.clone();
    same_line.push(if hanging {
        fmt.flat(head.clone())
    } else {
        head.clone()
    });
    same_line.push(block.clone());

    let mut next_line = builder.clone();
    next_line.push(head);
    next_line.push(fmt.on_next_line(block));
    fmt.alt_frag(same_line.into_frag(), next_line.into_frag())
}

/// The expression of a block that can be written as `{ expr }`: no statements and no comments
fn single_line_block_expr(block: &BlockExpr) -> Option<Expr> {
    let stmt_list = block.stmt_list()?;
    let has_comment = stmt_list
        .syntax()
        .children_with_tokens()
        .any(|e| e.kind() == SyntaxKind::COMMENT);
    if stmt_list.statements().next().is_some() || has_comment || block.label().is_some() {
        return None;
    }
    stmt_list.tail_expr()
}

/// `{ inner }`, where `inner` is the expression from `single_line_block_expr`; the caller restricts it to a single line
fn format_block_single_line(fmt: &Formatter, block: &BlockExpr, inner: Frag) -> Frag {
    let stmt_list = block.stmt_list().unwrap();
    format_braced(
        fmt,
        stmt_list.l_curly_token().unwrap(),
        inner,
        stmt_list.r_curly_token().unwrap(),
    )
}

/// The expressions of the blocks of `if a { b } else { c }` that can be put on a single line:
/// a single `else` and blocks containing just an expression. Only a syntactic check, nothing is formatted.
fn single_line_if_exprs(expr: &IfExpr) -> Option<(Expr, Expr)> {
    let Some(ElseBranch::Block(else_block)) = expr.else_branch() else {
        return None;
    };
    Some((
        single_line_block_expr(&expr.then_branch()?)?,
        single_line_block_expr(&else_block)?,
    ))
}

/// `if a { b } else { c }` on a single line; `then_expr` and `else_expr` are the formatted
/// expressions from `single_line_if_exprs`. It is allowed only in an expression context
/// (not for a standalone statement).
fn format_if_single_line(
    fmt: &Formatter,
    expr: &IfExpr,
    condition: Frag,
    then_expr: Frag,
    else_expr: Frag,
) -> Option<Frag> {
    let Some(ElseBranch::Block(else_block)) = expr.else_branch() else {
        unreachable!("checked by `single_line_if_exprs`");
    };
    let mut builder = Builder::new(fmt);
    builder.push_keyword(expr.if_token()?);
    builder.push(condition);
    builder.push(format_block_single_line(
        fmt,
        &expr.then_branch()?,
        then_expr,
    ));
    builder.push_keyword(expr.else_token()?);
    builder.push(format_block_single_line(fmt, &else_block, else_expr));
    fmt.flat_within(
        builder.into_frag(),
        fmt.config().single_line_if_else_max_width,
    )
}

pub(super) fn format_if_expr(fmt: &Formatter, expr: IfExpr, allow_single_line: bool) -> Frag {
    let condition = format_expr(fmt, expr.condition().unwrap());
    let block_exprs = allow_single_line
        .then(|| single_line_if_exprs(&expr))
        .flatten()
        .map(|(then_expr, else_expr)| (format_expr(fmt, then_expr), format_expr(fmt, else_expr)));
    let (then_tail, else_tail) = block_exprs.clone().unzip();

    let mut builder = Builder::new(fmt);
    builder.push(format_head_with_block(
        fmt,
        expr.if_token().unwrap(),
        condition.clone(),
        is_hanging_expr(&expr.condition().unwrap()),
        format_block_expr_with_tail(fmt, expr.then_branch().unwrap(), then_tail),
    ));
    if let Some(else_branch) = expr.else_branch() {
        // `else` is not a continuation of the block: own-line comments in front of it keep the indentation of `}`
        builder.flush_trivia(false);
        builder.push_keyword(expr.else_token().unwrap());
        builder.push(match else_branch {
            ElseBranch::Block(block) => format_block_expr_with_tail(fmt, block, else_tail),
            ElseBranch::IfExpr(expr) => format_if_expr(fmt, expr, false),
        });
    }
    let frag = builder.into_frag();
    fmt.alt_opt(
        frag,
        block_exprs.and_then(|(then_expr, else_expr)| {
            format_if_single_line(fmt, &expr, condition, then_expr, else_expr)
        }),
    )
}

pub(super) fn format_expr(fmt: &Formatter, expr: Expr) -> Frag {
    match expr {
        Expr::Literal(expr) => format_literal(fmt, expr),
        Expr::CallExpr(expr) => format_call_expr(fmt, expr),
        Expr::PathExpr(expr) => format_path_expr(fmt, expr),
        Expr::BinExpr(expr) => format_bin_expr(fmt, expr),
        Expr::PrefixExpr(expr) => format_prefix_expr(fmt, expr),
        Expr::ParenExpr(expr) => format_paren_expr(fmt, expr),
        Expr::LetExpr(expr) => format_let_expr(fmt, expr),
        Expr::IfExpr(expr) => format_if_expr(fmt, expr, true),
        Expr::BlockExpr(expr) => format_block_expr(fmt, expr),
        x => {
            panic! {
                "{x:?}"
            }
        }
    }
}
