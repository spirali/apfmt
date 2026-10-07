//! Expressions

use super::item::closure_params_layouts;
use super::pat::format_pat;
use super::path::{format_generic_arg_list, format_name_ref, format_path};
use super::stmt::{format_block_expr, format_block_expr_with_tail, push_block_prefix};
use super::ty::{format_ret_type, format_type};
use crate::format::common::{format_lifetime, format_repeat, push_modifiers};
use crate::formatter::Edge::{BraceClose, BraceOpen, Operator, Tight, Word};
use crate::formatter::{Builder, Formatter, Frag};
use crate::layout::chain::{ChainElement, infix_chain, postfix_chain_layouts};
use crate::layout::list::TrailingSeparator;
use crate::layout::list::{
    Delimiters, FlatLimit, ListItem, ListLayout, Overflow, delimited_list, list_items_cast,
};
use crate::utils::with_is_last;
use ra_ap_syntax::ast::{
    ArgList, ArrayExpr, BinExpr, BinaryOp, BlockExpr, BreakExpr, CallExpr, CastExpr, ClosureExpr,
    ContinueExpr, ElseBranch, Expr, HasArgList, HasAttrs, HasGenericArgs, IfExpr, LetExpr, Literal,
    ParenExpr, Pat, PathExpr, PrefixExpr, RangeExpr, RangeItem, RefExpr, ReturnExpr, TupleExpr,
};
use ra_ap_syntax::{AstNode, SyntaxKind, SyntaxNode, SyntaxToken};

fn format_path_expr(fmt: &Formatter, path_expr: PathExpr) -> Frag {
    format_path(fmt, path_expr.path().unwrap())
}

/// Whether a single argument can overflow, i.e. start on the line of `(` and end on the line of `)`:
/// `f(if a {\n ...\n})`. Follows the combinable expressions of the style guide.
// TODO: `match`, loops, struct literals and macros once they are supported
fn is_combinable_expr(expr: &Expr) -> bool {
    match expr {
        Expr::CallExpr(_)
        | Expr::MethodCallExpr(_)
        | Expr::IfExpr(_)
        | Expr::BlockExpr(_)
        | Expr::ArrayExpr(_)
        | Expr::TupleExpr(_)
        | Expr::ClosureExpr(_) => true,
        // `f(!g(\n...\n))`, `f(&[\n...\n])`
        Expr::PrefixExpr(expr) => expr.expr().is_some_and(|e| is_combinable_expr(&e)),
        Expr::RefExpr(expr) => expr.expr().is_some_and(|e| is_combinable_expr(&e)),
        Expr::CastExpr(expr) => expr.expr().is_some_and(|e| is_combinable_expr(&e)),
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
fn is_fillable_list(fmt: &Formatter, exprs: &[Expr], items: &[ListItem]) -> bool {
    let threshold = fmt.config().short_array_element_width_threshold as usize;
    threshold > 0
        && exprs.iter().all(is_simple_expr)
        && items
            .iter()
            .all(|i| fmt.flat_width(&i.frag).is_some_and(|w| w <= threshold))
}

/// Whether the last of more expressions may overflow (Style Guide): it is a closure and no other one is
fn is_overflowing_closure(exprs: &[Expr]) -> bool {
    let is_closure = |e: &Expr| matches!(e, Expr::ClosureExpr(_));
    match exprs {
        [rest @ .., last] => !rest.is_empty() && is_closure(last) && !rest.iter().any(is_closure),
        [] => false,
    }
}

/// An expression as a list item, and how it may overflow as the last item (see `Overflow`): a
/// combinable expression with all its layouts (see `is_combinable_expr`), a method call chain and
/// a closure with some of them (see `postfix_chain_layouts` and `ClosureLayouts`)
fn format_expr_item(fmt: &Formatter, expr: Expr) -> (Frag, Overflow) {
    match expr {
        // On the line of `(` only with its elements on one line, the arguments of the last one may
        // break: `f(a.b(|x| {` / `...` / `}))`, not the chain broken before every `.`
        Expr::MethodCallExpr(ref call) if call.attrs().next().is_none() => {
            let layouts = postfix_chain_parts(fmt, expr);
            let overflow = layouts
                .other
                .clone()
                .map_or(Overflow::Never, Overflow::With);
            (layouts.all(fmt), overflow)
        }
        Expr::ClosureExpr(closure) if closure.attrs().next().is_none() => {
            let layouts = closure_layouts(fmt, closure);
            let overflow = layouts
                .overflow
                .map_or(Overflow::Never, Overflow::Preferred);
            (layouts.all, overflow)
        }
        expr => {
            let combinable = is_combinable_expr(&expr);
            let frag = format_expr(fmt, expr);
            let overflow = if combinable {
                Overflow::With(frag.clone())
            } else {
                Overflow::Never
            };
            (frag, overflow)
        }
    }
}

/// The expressions of the comma-separated list `node` (call arguments, tuple fields, array elements)
/// on a single line if they fit within `width`, otherwise one per line, or filled on lines if they
/// are short (see `is_fillable_list`). A single combinable expression may overflow (see
/// `is_combinable_expr`), and so may a closure after other arguments (`f(a, |x| {` / `...` / `})`);
/// a single other expression is limited only by `max_width`.
fn format_expr_list(
    fmt: &Formatter,
    node: &SyntaxNode,
    delims: &Delimiters,
    width: u32,
    trailing: TrailingSeparator,
) -> Frag {
    let exprs = node.children().filter_map(Expr::cast).collect::<Vec<_>>();
    let single = match exprs.as_slice() {
        [single] => Some(single),
        _ => None,
    };
    let hug = single.is_some_and(is_combinable_expr) || is_overflowing_closure(&exprs);
    let items = list_items_cast(node, |e| format_expr_item(fmt, e));
    let items = with_is_last(&items)
        .map(|(item, is_last)| {
            let (frag, overflow) = &item.frag;
            let overflow = if hug && is_last {
                overflow.clone()
            } else {
                Overflow::Never
            };
            item.map(|_| frag.clone()).with_overflow(overflow)
        })
        .collect::<Vec<ListItem>>();
    // A single argument that cannot overflow is not limited by `fn_call_width`, nor is a closure.
    // Another one that can (e.g. a call) is: when it does not fit, it is broken instead, like
    // rustfmt does.
    let unlimited = single.is_some_and(|e| !hug || matches!(e, Expr::ClosureExpr(_)));
    let flat = if unlimited {
        FlatLimit::MaxWidth
    } else {
        FlatLimit::Within(width)
    };
    let layout = ListLayout::tall(flat)
        .fill_if(is_fillable_list(fmt, &exprs, &items))
        .trailing_separator(trailing);
    delimited_list(fmt, delims, &items, layout)
}

fn format_arg_list(fmt: &Formatter, arg_list: ArgList) -> Frag {
    format_expr_list(
        fmt,
        arg_list.syntax(),
        &Delimiters::parens_of(arg_list.syntax()),
        fmt.config().fn_call_width,
        TrailingSeparator::IfVertical,
    )
}

/// `(a, b)`, or `(a,)` with a single field; laid out as call arguments
fn format_tuple_expr(fmt: &Formatter, expr: TupleExpr) -> Frag {
    format_expr_list(
        fmt,
        expr.syntax(),
        &Delimiters::parens_of(expr.syntax()),
        fmt.config().fn_call_width,
        single_item_separator(expr.fields().count()),
    )
}

/// The trailing separator of a tuple: `(a,)` is a tuple, `(a)` just a parenthesized expression
pub(super) fn single_item_separator(items: usize) -> TrailingSeparator {
    if items == 1 {
        TrailingSeparator::Required
    } else {
        TrailingSeparator::IfVertical
    }
}

/// `[a, b, c]` laid out as call arguments within `array_width`, or `[value; len]`
fn format_array_expr(fmt: &Formatter, expr: ArrayExpr) -> Frag {
    let Some(semicolon) = expr.semicolon_token() else {
        return format_expr_list(
            fmt,
            expr.syntax(),
            &Delimiters::brackets_of(expr.syntax()),
            fmt.config().array_width,
            TrailingSeparator::IfVertical,
        );
    };
    let mut exprs = expr.exprs();
    let value = format_expr(fmt, exprs.next().unwrap());
    let len = format_expr(fmt, exprs.next().unwrap());
    format_repeat(fmt, expr.syntax(), value, semicolon, len)
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
    if let Some(BinaryOp::Assignment { .. }) = expr.op_kind() {
        return format_assignment(fmt, expr);
    }
    let mut rest = Vec::new();
    let first = format_expr(fmt, collect_bin_chain(expr, &mut rest));
    let rest = rest
        .into_iter()
        .map(|(op, e)| (fmt.token(op, Operator), format_expr(fmt, e)))
        .collect::<Vec<_>>();
    infix_chain(fmt, first, &rest)
}

/// `lhs = rhs`, `lhs += rhs`, ...: broken after the operator, like `let`
fn format_assignment(fmt: &Formatter, expr: BinExpr) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_expr(fmt, expr.lhs().unwrap()));
    builder.push_token(expr.op_token().unwrap(), Operator);
    builder.push(fmt.same_or_indented_line(format_expr(fmt, expr.rhs().unwrap())));
    builder.into_frag()
}

fn format_prefix_expr(fmt: &Formatter, expr: PrefixExpr) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_token(expr.op_token().unwrap(), Tight);
    builder.push(format_expr(fmt, expr.expr().unwrap()));
    builder.into_frag()
}

fn format_paren_expr(fmt: &Formatter, expr: ParenExpr) -> Frag {
    let inner = format_expr(fmt, expr.expr().unwrap());
    Delimiters::parens_of(expr.syntax()).enclose(fmt, inner)
}

/// `&x`, `&mut x`, `&raw const x`
fn format_ref_expr(fmt: &Formatter, expr: RefExpr) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_token(expr.amp_token().unwrap(), Tight);
    for token in [expr.raw_token(), expr.const_token(), expr.mut_token()]
        .into_iter()
        .flatten()
    {
        builder.push_keyword(token);
    }
    builder.push(format_expr(fmt, expr.expr().unwrap()));
    builder.into_frag()
}

/// `expr as Type`
fn format_cast_expr(fmt: &Formatter, expr: CastExpr) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_expr(fmt, expr.expr().unwrap()));
    builder.push_keyword(expr.as_token().unwrap());
    builder.push(format_type(fmt, expr.ty().unwrap()));
    builder.into_frag()
}

/// `a..b`, `a..=b`, `a..`, `..b` or `..`, without spaces
pub(super) fn format_range<R: RangeItem>(
    fmt: &Formatter,
    range: &R,
    format_bound: impl Fn(R::Bound) -> Frag,
) -> Frag {
    let mut builder = Builder::new(fmt);
    if let Some(start) = range.start() {
        builder.push(format_bound(start));
    }
    builder.push_token(range.op_token().unwrap(), Tight);
    if let Some(end) = range.end() {
        builder.push(format_bound(end));
    }
    builder.into_frag()
}

fn format_range_expr(fmt: &Formatter, expr: RangeExpr) -> Frag {
    format_range(fmt, &expr, |e| format_expr(fmt, e))
}

/// A keyword optionally followed by a label and a value: `return x`, `break 'a x`, `continue 'a`
fn format_jump(
    fmt: &Formatter,
    keyword: SyntaxToken,
    label: Option<Frag>,
    value: Option<Expr>,
) -> Frag {
    let mut builder = Builder::new(fmt);
    if label.is_some() || value.is_some() {
        builder.push_keyword(keyword);
    } else {
        builder.push_token(keyword, Word);
    }
    if let Some(label) = label {
        builder.push(label);
    }
    if let Some(value) = value {
        builder.push(format_expr(fmt, value));
    }
    builder.into_frag()
}

fn format_return_expr(fmt: &Formatter, expr: ReturnExpr) -> Frag {
    format_jump(fmt, expr.return_token().unwrap(), None, expr.expr())
}

fn format_break_expr(fmt: &Formatter, expr: BreakExpr) -> Frag {
    let label = expr.lifetime().map(|l| format_lifetime(fmt, l));
    format_jump(fmt, expr.break_token().unwrap(), label, expr.expr())
}

fn format_continue_expr(fmt: &Formatter, expr: ContinueExpr) -> Frag {
    let label = expr.lifetime().map(|l| format_lifetime(fmt, l));
    format_jump(fmt, expr.continue_token().unwrap(), label, None)
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
/// `a && b` or `a.b()` / `    .c()`; in the head of a control-flow expression, `{` then has to go on its
/// own line (see `format_head_with_block`)
fn is_hanging_expr(expr: &Expr) -> bool {
    match expr {
        Expr::BinExpr(_) => true,
        expr if is_postfix_expr(expr) => true,
        // A broken or-pattern continues with `| B` lines
        Expr::LetExpr(expr) => {
            matches!(expr.pat(), Some(Pat::OrPat(_)))
                || expr.expr().is_some_and(|e| is_hanging_expr(&e))
        }
        Expr::PrefixExpr(expr) => expr.expr().is_some_and(|e| is_hanging_expr(&e)),
        _ => false,
    }
}

/// `keyword head {`: the head of a control-flow expression (`if cond`, `while cond`, `match expr`,
/// `for pat in expr`) followed by its block, see `format_head_followed_by`
fn format_head_with_block(
    fmt: &Formatter,
    keyword: SyntaxToken,
    head: &HeadLayouts,
    block: Frag,
) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_keyword(keyword);
    builder.push(format_head_followed_by(fmt, head, block, |frag| frag));
    builder.into_frag()
}

/// `head` followed by `block` (`{ ... }`, `else { ... }`), with `place` applied to each layout of the
/// head (e.g. the right-hand side of `=`). The block follows a head without a hanging indent on its
/// last line if it fits; otherwise the control line is broken and `{` goes on its own line (Style
/// Guide). Below a head broken with a hanging indent, it is always on its own line, so `{` is not
/// mistaken for a part of the head.
pub(super) fn format_head_followed_by(
    fmt: &Formatter,
    head: &HeadLayouts,
    block: Frag,
    place: impl Fn(Frag) -> Frag,
) -> Frag {
    let next_line = |head: Frag| {
        let mut builder = Builder::new(fmt);
        builder.push(head);
        builder.push(fmt.on_next_line(block.clone()));
        builder.into_frag()
    };
    let mut layouts = None;
    if let Some(other) = &head.other {
        let other = place(other.clone());
        let mut same_line = Builder::new(fmt);
        same_line.push(other.clone());
        same_line.push(block.clone());
        // `{` on its own line breaks the control line, so only when it does not fit otherwise: it
        // costs a split, which outweighs any number of lines (e.g. moving a comment from `{`'s line
        // into the block)
        let broken = fmt.dispreferred(next_line(other));
        layouts = Some(fmt.alt_frag(same_line.into_frag(), broken));
    }
    if let Some(hanging) = &head.hanging {
        layouts = Some(fmt.alt_opt(next_line(place(hanging.clone())), layouts));
    }
    layouts.unwrap()
}

/// The expression of a block that can be written as `{ expr }`: no statements and no comments
fn single_line_block_expr(block: &BlockExpr) -> Option<Expr> {
    let stmt_list = block.stmt_list()?;
    let has_comment = stmt_list
        .syntax()
        .children_with_tokens()
        .any(|e| e.kind() == SyntaxKind::COMMENT);
    if stmt_list.statements().next().is_some()
        || stmt_list.attrs().next().is_some()
        || has_comment
        || block.label().is_some()
    {
        return None;
    }
    stmt_list.tail_expr()
}

/// `{ inner }`, where `inner` is the expression from `single_line_block_expr`; the caller restricts it
/// to a single line
fn format_braced_block(fmt: &Formatter, block: &BlockExpr, inner: Frag) -> Frag {
    Delimiters::braces_of(block.stmt_list().unwrap().syntax()).enclose(fmt, inner)
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
    builder.push(format_braced_block(fmt, &expr.then_branch()?, then_expr));
    builder.push_keyword(expr.else_token()?);
    builder.push(format_braced_block(fmt, &else_block, else_expr));
    fmt.flat_within(
        builder.into_frag(),
        fmt.config().single_line_if_else_max_width,
    )
}

pub(super) fn format_if_expr(fmt: &Formatter, expr: IfExpr, allow_single_line: bool) -> Frag {
    let head = format_head_expr(fmt, expr.condition().unwrap());
    let block_exprs = allow_single_line
        .then(|| single_line_if_exprs(&expr))
        .flatten()
        .map(|(then_expr, else_expr)| (format_expr(fmt, then_expr), format_expr(fmt, else_expr)));
    let (then_tail, else_tail) = block_exprs.clone().unzip();

    let mut builder = Builder::new(fmt);
    builder.push(format_head_with_block(
        fmt,
        expr.if_token().unwrap(),
        &head,
        format_block_expr_with_tail(fmt, expr.then_branch().unwrap(), then_tail),
    ));
    if let Some(else_branch) = expr.else_branch() {
        // `else` is not a continuation of the block: own-line comments in front of it keep the indentation of `}`
        builder.flush_trivia();
        builder.push_keyword(expr.else_token().unwrap());
        builder.push(match else_branch {
            ElseBranch::Block(block) => format_block_expr_with_tail(fmt, block, else_tail),
            ElseBranch::IfExpr(expr) => format_if_expr(fmt, expr, false),
        });
    }
    let frag = builder.into_frag();
    fmt.alt_opt(
        frag,
        // A single line excludes the layouts of the condition with a hanging indent
        block_exprs.and_then(|(then_expr, else_expr)| {
            let condition = head.other.clone()?;
            format_if_single_line(fmt, &expr, condition, then_expr, else_expr)
        }),
    )
}

/// Splits a postfix chain `a.b()?.c[0]` into its root (`a`, the return value) and the postfix
/// expressions applied to it, innermost first
fn collect_postfix_chain(expr: Expr, postfix: &mut Vec<Expr>) -> Expr {
    let inner = match &expr {
        Expr::MethodCallExpr(e) => e.receiver(),
        Expr::FieldExpr(e) => e.expr(),
        Expr::AwaitExpr(e) => e.expr(),
        Expr::TryExpr(e) => e.expr(),
        Expr::IndexExpr(e) => e.base(),
        _ => return expr,
    };
    let root = collect_postfix_chain(inner.unwrap(), postfix);
    postfix.push(expr);
    root
}

/// `block` broken, or on a single line if it holds just an expression (see `single_line_block_expr`):
/// `unsafe { x }`
fn format_block_or_inline(fmt: &Formatter, block: BlockExpr) -> Frag {
    let Some(tail) = single_line_block_expr(&block) else {
        return format_block_expr(fmt, block);
    };
    let tail = format_expr(fmt, tail);
    let (inline, broken) = inline_and_broken_block(fmt, block, tail);
    fmt.alt_frag(inline, broken)
}

/// `block` on a single line and broken, `tail` being its formatted expression (see
/// `single_line_block_expr`)
fn inline_and_broken_block(fmt: &Formatter, block: BlockExpr, tail: Frag) -> (Frag, Frag) {
    let mut inline = Builder::new(fmt);
    push_block_prefix(fmt, &mut inline, &block);
    inline.push(format_braced_block(fmt, &block, tail.clone()));
    let broken = format_block_expr_with_tail(fmt, block, Some(tail));
    (fmt.flat(inline.into_frag()), broken)
}

/// The body of a closure that may be written without braces: the only expression of a plain block
/// `{ expr }` (no statements, comments, label or modifiers) of a closure without a return type, or a
/// body that is not a plain block
fn unbraced_closure_body(expr: &ClosureExpr) -> Option<Expr> {
    let body = expr.body()?;
    let Expr::BlockExpr(block) = &body else {
        return Some(body);
    };
    let stmt_list = block.stmt_list()?;
    // A label or a modifier in front of `{`: `unsafe {`
    if block.syntax().first_token() != stmt_list.l_curly_token() {
        return Some(body);
    }
    if expr.ret_type().is_some() {
        return None;
    }
    single_line_block_expr(block)
}

/// Whether a closure body without braces may span more lines, as in rustfmt: a block-like expression
/// (`|x| unsafe {` / `...` / `}`); other expressions get braces then (Style Guide: "Add the `{}` when
/// ... the body expression is a control-flow expression that spans multiple lines")
// TODO: `match`, loops and struct literals once they are supported
fn is_block_like_expr(expr: &Expr) -> bool {
    matches!(expr, Expr::BlockExpr(_))
}

fn format_closure_expr(fmt: &Formatter, expr: ClosureExpr) -> Frag {
    closure_layouts(fmt, expr).all
}

/// The layouts of a closure: all of them, and those that may overflow as the last argument of a call,
/// with the parameters on one line (the Style Guide combines "a multi-line closure with an explicit
/// block"; rustfmt does not combine a closure with broken parameters either)
struct ClosureLayouts {
    all: Frag,
    overflow: Option<Frag>,
}

/// `move |a, b| body` (Style Guide "Closures"), see `closure_body_layouts`. Parameters that do not fit
/// on one line are aligned after `|` (see `closure_params_layouts`); a body in braces then keeps them,
/// as in rustfmt.
fn closure_layouts(fmt: &Formatter, expr: ClosureExpr) -> ClosureLayouts {
    assert!(expr.for_binder().is_none(), "not supported yet: {expr:?}");
    let mut prefix = Builder::new(fmt);
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
    let inner = match unbraced_closure_body(expr) {
        // Braces required, or the body is a block itself: `unsafe {`
        Some(inner) if inner.syntax() != body.syntax() || !is_block_like_expr(&inner) => inner,
        _ => {
            let Expr::BlockExpr(block) = body else {
                unreachable!("checked by `unbraced_closure_body` and `is_block_like_expr`");
            };
            let block = format_block_or_inline(fmt, block);
            return (block.clone(), Some(block));
        }
    };
    let block_like = is_block_like_expr(&inner);
    let inner_frag = format_expr(fmt, inner);
    let mut unbraced = Builder::new(fmt);
    unbraced.push(if block_like {
        inner_frag.clone()
    } else {
        fmt.flat(inner_frag.clone())
    });
    match body {
        // The braces of the source are removed
        Expr::BlockExpr(block) => {
            let close = block.stmt_list().unwrap().r_curly_token().unwrap();
            unbraced.skip_to_token(close);
            if !keep_braces {
                let braced = format_block_expr_with_tail(fmt, block, Some(inner_frag));
                return (fmt.alt_frag(unbraced.into_frag(), braced), None);
            }
            let (inline, braced) = inline_and_broken_block(fmt, block, inner_frag);
            let all = fmt.alt_frag(unbraced.into_frag(), braced.clone());
            (all, Some(fmt.alt_frag(inline, braced)))
        }
        // Braces are added
        _ => {
            let mut braced = Builder::new(fmt);
            braced.push_generated("{", BraceOpen);
            let mut line = Builder::new(fmt);
            line.push(inner_frag);
            line.push_newline();
            braced.push(fmt.body_after_opener(false, line.into_frag()));
            braced.push_generated("}", BraceClose);
            let all = fmt.alt_frag(unbraced.into_frag(), braced.into_frag());
            (all.clone(), Some(all))
        }
    }
}

/// A chain of fields, method calls, `.await`, `?` and indexes, see `postfix_chain_layouts`
fn format_postfix_chain(fmt: &Formatter, expr: Expr) -> Frag {
    postfix_chain_parts(fmt, expr).all(fmt)
}

/// The layouts of a postfix chain, see `postfix_chain_layouts`. `?` and indexes stay with the element
/// (or root) in front of them: `a.b()?[0]`.
fn postfix_chain_parts(fmt: &Formatter, expr: Expr) -> HeadLayouts {
    let mut postfix = Vec::new();
    let root_expr = collect_postfix_chain(expr, &mut postfix);
    let mut root = Builder::new(fmt);
    root.push(format_expr(fmt, root_expr));
    let mut elements: Vec<(Builder, Builder)> = Vec::new();
    for expr in postfix {
        let mut head = Builder::new(fmt);
        match expr {
            Expr::MethodCallExpr(e) => {
                head.push_token(e.dot_token().unwrap(), Tight);
                head.push(format_name_ref(fmt, e.name_ref().unwrap()));
                if let Some(args) = e.generic_arg_list() {
                    head.push(format_generic_arg_list(fmt, args));
                }
                let mut tail = Builder::new(fmt);
                tail.push(format_arg_list(fmt, e.arg_list().unwrap()));
                elements.push((head, tail));
            }
            Expr::FieldExpr(e) => {
                head.push_token(e.dot_token().unwrap(), Tight);
                head.push(format_name_ref(fmt, e.name_ref().unwrap()));
                elements.push((head, Builder::new(fmt)));
            }
            Expr::AwaitExpr(e) => {
                head.push_token(e.dot_token().unwrap(), Tight);
                head.push_token(e.await_token().unwrap(), Word);
                elements.push((head, Builder::new(fmt)));
            }
            Expr::TryExpr(e) => {
                let last = elements.last_mut().map_or(&mut root, |(_, tail)| tail);
                last.push_token(e.question_mark_token().unwrap(), Tight);
            }
            Expr::IndexExpr(e) => {
                // Broken only when it does not fit on one line: `a[i + j]`
                let index = fmt.keep_together(format_expr(fmt, e.index().unwrap()));
                let index = Delimiters::brackets_of(e.syntax()).enclose(fmt, index);
                let last = elements.last_mut().map_or(&mut root, |(_, tail)| tail);
                last.push(index);
            }
            _ => unreachable!("not a postfix expression: {expr:?}"),
        }
    }
    let elements = elements
        .into_iter()
        .map(|(head, tail)| ChainElement {
            head: head.into_frag(),
            tail: tail.into_frag(),
        })
        .collect::<Vec<_>>();
    let (hanging, other) = postfix_chain_layouts(fmt, root.into_frag(), &elements);
    HeadLayouts { hanging, other }
}

/// Whether the expression is a postfix chain, see `format_postfix_chain`
fn is_postfix_expr(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::MethodCallExpr(_)
            | Expr::FieldExpr(_)
            | Expr::AwaitExpr(_)
            | Expr::TryExpr(_)
            | Expr::IndexExpr(_)
    )
}

/// The layouts of the head of a control-flow expression (`if cond`) or of the initializer of a
/// `let`-`else`, by what may follow them on their last line (at least one is present)
pub(super) struct HeadLayouts {
    /// The layouts ending with an indented continuation line (a hanging indent, see
    /// `is_hanging_expr`): `{` (`else {`) goes on its own line after them
    hanging: Option<Frag>,
    /// The other layouts: `{` may follow on their last line, e.g. after a chain whose last element
    /// breaks: `if a.b(|x| {` / `...` / `}) {`
    other: Option<Frag>,
}

impl HeadLayouts {
    fn all(self, fmt: &Formatter) -> Frag {
        fmt.alt_some(self.hanging, self.other)
            .expect("a head has a layout")
    }
}

pub(super) fn format_head_expr(fmt: &Formatter, expr: Expr) -> HeadLayouts {
    if is_postfix_expr(&expr) {
        return postfix_chain_parts(fmt, expr);
    }
    // `!a.b()`
    if let Expr::PrefixExpr(prefix) = &expr
        && let Some(operand) = prefix.expr().filter(is_postfix_expr)
    {
        let op = fmt.token(prefix.op_token().unwrap(), Tight);
        let head = format_head_expr(fmt, operand);
        let with_op = |frag: Frag| Builder::concat(fmt, [op.clone(), frag]);
        return HeadLayouts {
            hanging: head.hanging.map(with_op),
            other: head.other.map(with_op),
        };
    }
    let hanging = is_hanging_expr(&expr);
    let frag = format_expr(fmt, expr);
    if hanging {
        HeadLayouts {
            // Broken only when the head does not fit on its line: `if a && b` / `{` comes first
            hanging: Some(fmt.dispreferred(fmt.multi_line(frag.clone()))),
            other: Some(fmt.flat(frag)),
        }
    } else {
        HeadLayouts {
            hanging: None,
            other: Some(frag),
        }
    }
}

pub(super) fn format_expr(fmt: &Formatter, expr: Expr) -> Frag {
    if let Some(attr) = expr
        .syntax()
        .children()
        .find(|c| c.kind() == SyntaxKind::ATTR)
    {
        // It would be lost
        panic!("not supported yet: {attr:?}");
    }
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
        Expr::RefExpr(expr) => format_ref_expr(fmt, expr),
        Expr::CastExpr(expr) => format_cast_expr(fmt, expr),
        Expr::RangeExpr(expr) => format_range_expr(fmt, expr),
        Expr::TupleExpr(expr) => format_tuple_expr(fmt, expr),
        Expr::ArrayExpr(expr) => format_array_expr(fmt, expr),
        Expr::ReturnExpr(expr) => format_return_expr(fmt, expr),
        Expr::BreakExpr(expr) => format_break_expr(fmt, expr),
        Expr::ContinueExpr(expr) => format_continue_expr(fmt, expr),
        Expr::UnderscoreExpr(expr) => fmt.token(expr.underscore_token().unwrap(), Word),
        expr if is_postfix_expr(&expr) => format_postfix_chain(fmt, expr),
        Expr::ClosureExpr(expr) => format_closure_expr(fmt, expr),
        x => panic!("not supported yet: {x:?}"),
    }
}
