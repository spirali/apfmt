//! Expressions

use super::attr::{debug_assert_attrs_placed, format_skippable_with_attrs, format_with_attrs};
use super::control::{format_for_expr, format_loop_expr, format_match_expr, format_while_expr};
use super::generics::format_for_binder;
use super::item::closure_params_layouts;
use super::macros::{format_macro_call, is_list_macro};
use super::pat::{format_pat, pat_layouts};
use super::path::{format_generic_arg_list, format_name_ref, format_path};
use super::stmt::{format_block_expr, format_block_expr_with_tail, push_block_prefix};
use super::ty::{format_ret_type, format_type};
use super::{format_unsupported_kind, format_unsupported_with_attrs};
use crate::doc::{CLOSE_BRACE_DOC, OPEN_BRACE_DOC};
use crate::format::common::{format_lifetime, format_repeat, push_modifiers};
use crate::formatter::Edge::{BraceClose, BraceOpen, Comma, Operator, Tight, Word};
use crate::formatter::{Builder, Formatter, Frag};
use crate::layout::chain::{ChainElement, broken_infix_chain, infix_chain, postfix_chain_layouts};
use crate::layout::head::HeadLayouts;
use crate::layout::list::TrailingSeparator;
use crate::layout::list::{
    Delimiters, FlatLimit, ListItem, ListLayout, Overflow, delimited_list, list_items_cast,
};
use crate::utils::{has_attrs, is_brace_macro_expr, with_is_last};
use ra_ap_syntax::ast::{
    ArgList, ArrayExpr, BinExpr, BinaryOp, BlockExpr, BreakExpr, CallExpr, CastExpr, ClosureExpr,
    ContinueExpr, ElseBranch, Expr, HasArgList, HasAttrs, HasGenericArgs, IfExpr, IndexExpr,
    LetExpr, Literal, LogicOp, ParenExpr, PathExpr, PrefixExpr, RangeExpr, RangeItem, RecordExpr,
    RecordExprField, RefExpr, ReturnExpr, StmtList, TupleExpr,
};
use ra_ap_syntax::{AstNode, SyntaxKind, SyntaxNode, SyntaxToken};

fn format_path_expr(fmt: &Formatter, path_expr: PathExpr) -> Frag {
    format_path(fmt, path_expr.path().unwrap())
}

/// Whether a single argument can overflow, i.e. start on the line of `(` and end on the line of `)`:
/// `f(if a {\n ...\n})`. Follows the combinable expressions of the style guide.
fn is_combinable_expr(expr: &Expr) -> bool {
    match expr {
        Expr::CallExpr(_)
        | Expr::MethodCallExpr(_)
        | Expr::IfExpr(_)
        | Expr::BlockExpr(_)
        | Expr::ArrayExpr(_)
        | Expr::TupleExpr(_)
        | Expr::ClosureExpr(_)
        | Expr::RecordExpr(_)
        | Expr::MatchExpr(_) => true,
        expr if is_list_macro(expr) => true,
        // `f(!g(\n...\n))`, `f(&[\n...\n])`
        Expr::PrefixExpr(expr) => expr.expr().is_some_and(|e| is_combinable_expr(&e)),
        Expr::RefExpr(expr) => expr.expr().is_some_and(|e| is_combinable_expr(&e)),
        Expr::CastExpr(expr) => expr.expr().is_some_and(|e| is_combinable_expr(&e)),
        // `f(g(\n...\n)?)`, as rustfmt does
        Expr::TryExpr(expr) => expr.expr().is_some_and(|e| is_combinable_expr(&e)),
        _ => false,
    }
}

/// The expression without the `?` that follow it: `a` of `a??`
fn without_try(expr: &Expr) -> Expr {
    match expr {
        Expr::TryExpr(e) => e.expr().map_or_else(|| expr.clone(), |e| without_try(&e)),
        _ => expr.clone(),
    }
}

/// Simple expressions in the sense of rustfmt's `is_simple_expr`: literals, plain names, and simple
/// expressions after a unary operator, `&` or `&mut`, followed by a field, `?` or `as Type`, indexed by
/// a simple expression (`a[i]`) or repeated (`[a; n]`)
pub(super) fn is_simple_expr(expr: &Expr) -> bool {
    let is_simple = |expr: Option<Expr>| expr.is_some_and(|e| is_simple_expr(&e));
    match expr {
        Expr::Literal(_) | Expr::PathExpr(_) => is_literal_or_name(expr),
        Expr::PrefixExpr(expr) => is_simple(expr.expr()),
        Expr::RefExpr(expr) => is_simple(expr.expr()),
        Expr::CastExpr(expr) => is_simple(expr.expr()),
        Expr::FieldExpr(expr) => is_simple(expr.expr()),
        Expr::TryExpr(expr) => is_simple(expr.expr()),
        Expr::IndexExpr(expr) => is_simple(expr.base()) && is_simple(expr.index()),
        Expr::ArrayExpr(expr) if expr.semicolon_token().is_some() => {
            expr.exprs().all(|e| is_simple_expr(&e))
        }
        _ => false,
    }
}

/// A literal or a plain name, optionally after unary operators: `x`, `!x`, `-1`
fn is_literal_or_name(expr: &Expr) -> bool {
    match expr {
        Expr::Literal(_) => true,
        Expr::PathExpr(expr) => expr.path().is_some_and(|path| {
            path.qualifier().is_none()
                && path
                    .segment()
                    .is_some_and(|s| s.type_anchor().is_none() && s.coloncolon_token().is_none())
        }),
        Expr::PrefixExpr(expr) => expr.expr().is_some_and(|e| is_literal_or_name(&e)),
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

/// Whether the last of more expressions may overflow: a closure if no other one is (Style Guide), or
/// a block, also `unsafe`, `async` or labelled (`f(a, unsafe {` / `...` / `})`, as rustfmt does)
fn is_overflowing_last(exprs: &[Expr]) -> bool {
    let is_closure = |e: &Expr| matches!(e, Expr::ClosureExpr(_));
    match exprs {
        [rest @ .., last] if !rest.is_empty() => {
            matches!(last, Expr::BlockExpr(_)) || is_closure(last) && !rest.iter().any(is_closure)
        }
        _ => false,
    }
}

/// An expression as a list item, and how it may overflow as the last item (see `Overflow`): a
/// combinable expression with all its layouts (see `is_combinable_expr`), a method call chain and
/// a closure with some of them (see `postfix_chain_layouts` and `ClosureLayouts`)
fn format_expr_item(fmt: &Formatter, expr: Expr) -> (Frag, Overflow) {
    match expr {
        // On the line of `(` only with its elements on one line, the arguments of the last one may
        // break: `f(a.b(|x| {` / `...` / `}))`, `f(a.b(` / `...` / `)?)`, not the chain broken
        // before every `.`; `f(g(` / `...` / `)?)` is the root of a chain without elements
        Expr::MethodCallExpr(_) | Expr::TryExpr(_)
            if !has_attrs(&expr) && is_combinable_expr(&expr) =>
        {
            let layouts = postfix_chain_parts(fmt, expr);
            let overflow = layouts
                .other
                .clone()
                .map_or(Overflow::Never, Overflow::With);
            (layouts.all(fmt), overflow)
        }
        Expr::ClosureExpr(closure) if !has_attrs(&closure) => {
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
/// `is_combinable_expr`), and so may a closure or a block after other arguments (`f(a, |x| {` /
/// `...` / `})`, see `is_overflowing_last`);
/// a single other expression is limited only by `max_width`. `refine` adjusts the layouts, e.g. the
/// trailing separator.
pub(super) fn format_expr_list(
    fmt: &Formatter,
    node: &SyntaxNode,
    delims: &Delimiters,
    width: u32,
    refine: impl FnOnce(ListLayout) -> ListLayout,
) -> Frag {
    let exprs = node.children().filter_map(Expr::cast).collect::<Vec<_>>();
    let single = match exprs.as_slice() {
        [single] => Some(single),
        _ => None,
    };
    let hug = single.is_some_and(is_combinable_expr) || is_overflowing_last(&exprs);
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
    // A single argument that cannot overflow is not limited by `fn_call_width`, nor is a closure or
    // a method call (the only postfix chains that can overflow are method calls and combinable
    // expressions followed by `?`, see `is_combinable_expr`; the others, e.g. `a.b`, cannot).
    // Another one that can (e.g. a call) is: when it does not fit, it is broken instead. All as
    // rustfmt does; the Style Guide leaves what a small call is to the tool.
    let unlimited = single.is_some_and(|e| {
        !hug || matches!(
            without_try(e),
            Expr::ClosureExpr(_) | Expr::MethodCallExpr(_)
        )
    });
    let flat = if unlimited {
        FlatLimit::MaxWidth
    } else {
        FlatLimit::Within(width)
    };
    let layout = ListLayout::tall(flat).fill_if(is_fillable_list(fmt, &exprs, &items));
    delimited_list(fmt, delims, &items, refine(layout))
}

fn format_arg_list(fmt: &Formatter, arg_list: ArgList) -> Frag {
    format_expr_list(
        fmt,
        arg_list.syntax(),
        &Delimiters::parens_of(arg_list.syntax()),
        fmt.config().fn_call_width,
        |layout| layout,
    )
}

/// `(a, b)`, or `(a,)` with a single field; laid out as call arguments
fn format_tuple_expr(fmt: &Formatter, expr: TupleExpr) -> Frag {
    format_expr_list(
        fmt,
        expr.syntax(),
        &Delimiters::parens_of(expr.syntax()),
        fmt.config().fn_call_width,
        |layout| layout.trailing_separator(single_item_separator(expr.fields().count())),
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
pub(super) fn format_array_expr(fmt: &Formatter, expr: ArrayExpr) -> Frag {
    let Some(semicolon) = expr.semicolon_token() else {
        return format_expr_list(
            fmt,
            expr.syntax(),
            &Delimiters::brackets_of(expr.syntax()),
            fmt.config().array_width,
            |layout| layout,
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
    let (first, rest) = bin_chain_operands(fmt, expr);
    infix_chain(fmt, first, &rest)
}

/// The operands of a chain of binary operators formatted, for `infix_chain`
fn bin_chain_operands(fmt: &Formatter, expr: BinExpr) -> (Frag, Vec<(Frag, Frag)>) {
    let mut rest = Vec::new();
    let first = format_expr(fmt, collect_bin_chain(expr, &mut rest));
    let rest = rest
        .into_iter()
        .map(|(op, e)| (fmt.token(op, Operator), format_expr(fmt, e)))
        .collect();
    (first, rest)
}

/// `lhs = rhs`, `lhs += rhs`, ...: broken after the operator, like `let`
fn format_assignment(fmt: &Formatter, expr: BinExpr) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_expr(fmt, expr.lhs().unwrap()));
    builder.push_token(expr.op_token().unwrap(), Operator);
    builder.push(fmt.same_or_indented_line(format_expr(fmt, expr.rhs().unwrap())));
    builder.into_frag()
}

/// The operator of a prefix expression, its first token child; not `PrefixExpr::op_token`, its
/// first child, which is an attribute in `#[cfg(x)] -1`
fn prefix_op_token(expr: &PrefixExpr) -> SyntaxToken {
    expr.syntax()
        .children_with_tokens()
        .find_map(|e| e.into_token().filter(|t| !t.kind().is_trivia()))
        .unwrap()
}

fn format_prefix_expr(fmt: &Formatter, expr: PrefixExpr) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_token(prefix_op_token(&expr), Tight);
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
    let op = range.op_token().unwrap();
    match range.end() {
        Some(end) => {
            builder.push_token(op, Tight);
            builder.push(format_bound(end));
        }
        None => builder.push(fmt.postfix_token(op)),
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
        Expr::PrefixExpr(expr) => expr.expr().is_some_and(|e| is_hanging_expr(&e)),
        _ => false,
    }
}

/// `keyword head {`: the head of a control-flow expression (`if cond`, `while cond`, `match expr`,
/// `for pat in expr`) followed by its block, see `format_head_followed_by`
pub(super) fn format_head_with_block(
    fmt: &Formatter,
    keyword: SyntaxToken,
    head: &HeadLayouts,
    block: Frag,
) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_keyword(keyword);
    builder.push(format_head_followed_by(fmt, head, block));
    builder.into_frag()
}

/// `head` followed by `block` (`{ ... }`, `else { ... }`). The block follows a head without a hanging indent on its
/// last line if it fits; otherwise the control line is broken and `{` goes on its own line (Style
/// Guide). Below a head broken with a hanging indent, it is always on its own line, so `{` is not
/// mistaken for a part of the head.
pub(super) fn format_head_followed_by(fmt: &Formatter, head: &HeadLayouts, block: Frag) -> Frag {
    let next_line = |head: Frag| {
        let mut builder = Builder::new(fmt);
        builder.push(head);
        builder.push(fmt.on_next_line(block.clone()));
        builder.into_frag()
    };
    let mut layouts = None;
    if let Some(other) = &head.other {
        let mut same_line = Builder::new(fmt);
        same_line.push(other.clone());
        same_line.push(block.clone());
        // `{` on its own line breaks the control line, so only when it does not fit otherwise: it
        // costs a split, which outweighs any number of lines (e.g. moving a comment from `{`'s line
        // into the block)
        let broken = fmt.dispreferred(next_line(other.clone()));
        layouts = Some(fmt.alt_frag(same_line.into_frag(), broken));
    }
    if let Some(hanging) = &head.hanging {
        layouts = Some(fmt.alt_opt(next_line(hanging.clone()), layouts));
    }
    layouts.unwrap()
}

/// The statement list of a plain block: no label or modifier in front of `{` (`unsafe {` is an
/// expression)
pub(super) fn plain_stmt_list(block: &BlockExpr) -> Option<StmtList> {
    let stmt_list = block.stmt_list()?;
    (block.syntax().first_token() == stmt_list.l_curly_token()).then_some(stmt_list)
}

/// A plain block, or the innermost of nested plain blocks each holding just the next one,
/// `{ { expr } }`: the outer braces are removed (as in rustfmt), the innermost ones are kept or
/// removed as those of a single block
pub(super) struct NestedPlainBlocks {
    /// The innermost block
    pub inner: BlockExpr,
    /// The `}` of the outermost block
    close: SyntaxToken,
    nested: bool,
}

impl NestedPlainBlocks {
    /// `block` and the plain blocks nested in it (`block` must be a plain one, see `plain_stmt_list`)
    pub fn of(block: BlockExpr) -> Self {
        let close = block.stmt_list().unwrap().r_curly_token().unwrap();
        let mut inner = block;
        let mut nested = false;
        while let Some(Expr::BlockExpr(next)) = single_line_block_expr(&inner)
            && plain_stmt_list(&next).is_some()
        {
            inner = next;
            nested = true;
        }
        NestedPlainBlocks {
            inner,
            close,
            nested,
        }
    }

    /// A layout of the innermost block (or of its expression in braces), the braces around it skipped
    pub fn skip_outer_braces(&self, fmt: &Formatter, frag: Frag) -> Frag {
        if self.nested {
            Builder::skip_to_token_after(fmt, frag, self.close.clone())
        } else {
            frag
        }
    }

    /// The expression of the innermost block without braces, all the braces skipped
    pub fn skip_braces(&self, fmt: &Formatter, frag: Frag) -> Frag {
        Builder::skip_to_token_after(fmt, frag, self.close.clone())
    }
}

/// The expression of a block that can be written as `{ expr }`: no statements, no comments and no
/// attributes (`{` / `    #[cfg(x)]` / `    expr` / `}`); not a `try` block, which is copied (see `format_block_expr_with_tail`).
/// Not a macro call with braces, which is a statement there (`is_brace_macro_expr`): its block is
/// broken (as in rustfmt) and keeps its braces as a closure or match arm body.
pub(super) fn single_line_block_expr(block: &BlockExpr) -> Option<Expr> {
    let stmt_list = block.stmt_list()?;
    let has_comment = stmt_list
        .syntax()
        .children_with_tokens()
        .any(|e| e.kind() == SyntaxKind::COMMENT);
    let tail = stmt_list.tail_expr();
    if stmt_list.statements().next().is_some()
        || stmt_list.attrs().next().is_some()
        || tail.as_ref().is_some_and(has_attrs)
        || has_comment
        || block.try_block_modifier().is_some()
    {
        return None;
    }
    tail.filter(|expr| !is_brace_macro_expr(expr.syntax()))
}

/// `{ inner }`, where `inner` is the expression from `single_line_block_expr`; the caller restricts it
/// to a single line
pub(super) fn format_braced_block(fmt: &Formatter, block: &BlockExpr, inner: Frag) -> Frag {
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
    debug_assert_attrs_placed(fmt, &expr);
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

/// `[index]` of `target[index]`, broken only when it does not fit on one line, and then block
/// indented between the brackets, never before `[` (Style Guide "Array accesses, indexing, and
/// slicing"): `target[` / `    index` / `]`
fn format_index(fmt: &Formatter, expr: &IndexExpr) -> Frag {
    let index = ListItem::new(format_expr(fmt, expr.index().unwrap()));
    let layout =
        ListLayout::tall(FlatLimit::MaxWidth).trailing_separator(TrailingSeparator::Omitted);
    let delims = Delimiters::brackets_of(expr.syntax());
    fmt.keep_together(delimited_list(fmt, &delims, &[index], layout))
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
/// `unsafe { x }`; a block in expression position (Style Guide "Blocks", see `format_expr_at` for
/// statement position)
pub(super) fn format_block_or_inline(fmt: &Formatter, block: BlockExpr) -> Frag {
    debug_assert_attrs_placed(fmt, &block);
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

/// `move |a, b| body` or `for<'a> |a: &'a T| body` (Style Guide "Closures"), see `closure_body_layouts`. Parameters that do not fit
/// on one line are aligned after `|` (see `closure_params_layouts`); a body in braces then keeps them,
/// as in rustfmt.
fn closure_layouts(fmt: &Formatter, expr: ClosureExpr) -> ClosureLayouts {
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
            let all = fmt.alt_frag(unbraced, format_added_braces(fmt, inner_frag));
            (all.clone(), Some(all))
        }
    }
}

/// `{` / `    inner` / `}` with braces that are not in the source, e.g. around a closure body or a match
/// arm body that does not fit on the line
pub(super) fn format_added_braces(fmt: &Formatter, inner: Frag) -> Frag {
    let mut braced = Builder::new(fmt);
    braced.push_generated(OPEN_BRACE_DOC, BraceOpen);
    let mut line = Builder::new(fmt);
    line.push(inner);
    line.push_newline();
    braced.push(fmt.body_after_opener(false, line.into_frag()));
    braced.push_generated(CLOSE_BRACE_DOC, BraceClose);
    braced.into_frag()
}

/// `Path { a, b: c, ..base }` (Style Guide "Struct literals"): on a single line if the fields fit
/// within `struct_lit_width`, otherwise one field per line with a trailing comma; `..base` is last and
/// has no comma
fn format_record_expr(fmt: &Formatter, expr: RecordExpr) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_path(fmt, expr.path().unwrap()));
    let list = expr.record_expr_field_list().unwrap();
    let mut fields = list_items_cast::<RecordExprField, _>(list.syntax(), |field| {
        format_with_attrs(fmt, &field, false, || format_record_expr_field(fmt, &field))
    });
    let trailing = match list.dotdot_token() {
        Some(dotdot) => {
            let mut spread = Builder::new(fmt);
            spread.push_token(dotdot, Tight);
            if let Some(base) = list.spread() {
                spread.push(format_expr(fmt, base));
            }
            fields.push(ListItem::new(spread.into_frag()));
            TrailingSeparator::Omitted
        }
        None => TrailingSeparator::IfVertical,
    };
    let layout = ListLayout::tall(FlatLimit::Within(fmt.config().struct_lit_width))
        .trailing_separator(trailing);
    let delims = Delimiters::braces_of(list.syntax());
    builder.push(delimited_list(fmt, &delims, &fields, layout));
    builder.into_frag()
}

/// `name: value`, the value on the next line only if it overflows on the line of the name (RhsBreak, as
/// rustfmt), or `name` (the shorthand)
fn format_record_expr_field(fmt: &Formatter, field: &RecordExprField) -> Frag {
    let value = format_expr(fmt, field.expr().unwrap());
    let Some(colon) = field.colon_token() else {
        return value;
    };
    let mut builder = Builder::new(fmt);
    builder.push(format_name_ref(fmt, field.name_ref().unwrap()));
    builder.push_token(colon, Comma);
    builder.push(fmt.same_or_indented_line_on_overflow(value));
    builder.into_frag()
}

/// A chain of fields, method calls, `.await`, `?` and indexes, see `postfix_chain_layouts`
fn format_postfix_chain(fmt: &Formatter, expr: Expr) -> Frag {
    postfix_chain_parts(fmt, expr).all(fmt)
}

/// The layouts of a postfix chain, see `postfix_chain_layouts`. `?` and indexes stay with the element
/// (or root) in front of them: `a.b()?[0]`.
fn postfix_chain_parts(fmt: &Formatter, expr: Expr) -> HeadLayouts {
    debug_assert_attrs_placed(fmt, &expr);
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
                last.push(fmt.postfix_token(e.question_mark_token().unwrap()));
            }
            Expr::IndexExpr(e) => {
                let index = format_index(fmt, &e);
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

/// `let pat = expr` of `if let` or `while let`: with the expression on the line of `=`, or broken
/// after `=` with the expression on an indented line, which hangs (Style Guide). The expression broken
/// over more lines costs a split in both, so it moves to the next line when it fits on one line there
/// (as in rustfmt). A broken pattern costs a split; a broken or-pattern hangs too (see
/// `pat_layouts`).
fn let_head_layouts(fmt: &Formatter, expr: &LetExpr) -> HeadLayouts {
    let pat = pat_layouts(fmt, expr.pat().unwrap());
    let all_pat = pat.clone().all(fmt);
    let let_token = fmt.keyword(expr.let_token().unwrap());
    let eq = fmt.token(expr.eq_token().unwrap(), Operator);
    let with_pat =
        |pat: Frag, rhs: Frag| Builder::concat(fmt, [let_token.clone(), pat, eq.clone(), rhs]);
    let rhs = format_head_expr(fmt, expr.expr().unwrap());
    let rhs = HeadLayouts {
        hanging: rhs.hanging.map(|frag| fmt.keep_together(frag)),
        other: rhs.other.map(|frag| fmt.keep_together(frag)),
    };
    let other = (rhs.other.clone())
        .zip(pat.other)
        .map(|(rhs, pat)| with_pat(pat, rhs));
    let mut hanging = rhs
        .hanging
        .clone()
        .map(|rhs| with_pat(all_pat.clone(), rhs));
    if let (Some(pat), Some(rhs)) = (pat.hanging, rhs.other.clone()) {
        hanging = fmt.alt_some(hanging, Some(with_pat(pat, rhs)));
    }
    let next_line = Builder::concat(
        fmt,
        [
            let_token.clone(),
            all_pat,
            eq.clone(),
            fmt.indented_line(rhs.all(fmt)),
        ],
    );
    let hanging = fmt.alt_some(hanging, Some(next_line));
    HeadLayouts { hanging, other }
}

/// Whether the expression is a let chain (`a && let Some(b) = c`) that the Style Guide breaks before
/// every `&&`: all except two clauses, a literal or a name (optionally after unary operators) and a
/// `let` clause, which may stay on one line
fn is_broken_let_chain(expr: &Expr) -> bool {
    let Expr::BinExpr(expr) = expr else {
        return false;
    };
    if expr.op_kind() != Some(BinaryOp::LogicOp(LogicOp::And)) {
        return false;
    }
    let mut rest = Vec::new();
    let first = collect_bin_chain(expr.clone(), &mut rest);
    let is_let = |e: &Expr| matches!(e, Expr::LetExpr(_));
    if !is_let(&first) && !rest.iter().any(|(_, e)| is_let(e)) {
        return false;
    }
    !matches!(rest.as_slice(), [(_, second)] if is_literal_or_name(&first) && is_let(second))
}

pub(super) fn format_head_expr(fmt: &Formatter, expr: Expr) -> HeadLayouts {
    // With attributes: copied unchanged with them, see `format_expr`
    if has_attrs(&expr) {
        return HeadLayouts {
            hanging: None,
            other: Some(format_expr(fmt, expr)),
        };
    }
    if is_postfix_expr(&expr) {
        return postfix_chain_parts(fmt, expr);
    }
    if let Expr::LetExpr(let_expr) = &expr {
        return let_head_layouts(fmt, let_expr);
    }
    // `!a.b()`
    if let Expr::PrefixExpr(prefix) = &expr
        && let Some(operand) = prefix.expr().filter(is_postfix_expr)
    {
        let op = fmt.token(prefix_op_token(prefix), Tight);
        let head = format_head_expr(fmt, operand);
        let with_op = |frag: Frag| Builder::concat(fmt, [op.clone(), frag]);
        return HeadLayouts {
            hanging: head.hanging.map(with_op),
            other: head.other.map(with_op),
        };
    }
    if is_broken_let_chain(&expr) {
        let Expr::BinExpr(expr) = expr else {
            unreachable!("checked by `is_broken_let_chain`");
        };
        let (first, rest) = bin_chain_operands(fmt, expr);
        return HeadLayouts {
            hanging: Some(broken_infix_chain(fmt, first, &rest)),
            other: None,
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

/// An expression; with outer attributes, it is copied unchanged (attributes are formatted only in
/// front of a statement or the tail expression of a block, see `format_expr_at`)
pub(super) fn format_expr(fmt: &Formatter, expr: Expr) -> Frag {
    if has_attrs(&expr) {
        return format_unsupported_with_attrs(fmt, expr.syntax(), "an attribute on an expression");
    }
    format_expr_without_attrs(fmt, expr)
}

/// Where an expression stands, which decides how its attributes and a block or an `if` are
/// formatted, see `format_expr_at`
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ExprPosition {
    /// An expression statement
    Statement,
    /// The tail expression of a block
    Tail,
    /// The body of a match arm
    ArmBody,
}

/// An expression at `position`. The attributes of a statement or a tail are on their own lines as
/// on a `let` statement (`#[cfg(x)]` / `if a {`, see `format_skippable_with_attrs`) and a block
/// after them is broken (as rustfmt does); elsewhere they are copied unchanged with the expression
/// (see `format_expr`). A standalone `if` (a statement, an arm body) is never on a single line, nor
/// is a statement block other than `unsafe { ... }` (Style Guide "Blocks": a single-line block is
/// "either used in expression position (not statement position) or is an unsafe block in statement
/// position"). The tail expression of a block is its value, so it is in expression position, as in
/// rustfmt.
pub(super) fn format_expr_at(fmt: &Formatter, expr: Expr, position: ExprPosition) -> Frag {
    use ExprPosition::{ArmBody, Statement, Tail};
    let has_attrs = has_attrs(&expr);
    if has_attrs && position == ArmBody {
        return format_expr(fmt, expr);
    }
    let format = || match expr.clone() {
        Expr::IfExpr(expr) if position != Tail => format_if_expr(fmt, expr, false),
        Expr::BlockExpr(block)
            if has_attrs || position == Statement && block.unsafe_token().is_none() =>
        {
            format_block_expr(fmt, block)
        }
        Expr::BlockExpr(block) if position == ArmBody => format_block_or_inline(fmt, block),
        expr => format_expr_without_attrs(fmt, expr),
    };
    if has_attrs {
        format_skippable_with_attrs(fmt, &expr, format)
    } else {
        format()
    }
}

/// An expression without its outer attributes, which the caller places (see
/// `debug_assert_attrs_placed`)
pub(super) fn format_expr_without_attrs(fmt: &Formatter, expr: Expr) -> Frag {
    debug_assert_attrs_placed(fmt, &expr);
    match expr {
        Expr::Literal(expr) => format_literal(fmt, expr),
        Expr::CallExpr(expr) => format_call_expr(fmt, expr),
        Expr::PathExpr(expr) => format_path_expr(fmt, expr),
        Expr::BinExpr(expr) => format_bin_expr(fmt, expr),
        Expr::PrefixExpr(expr) => format_prefix_expr(fmt, expr),
        Expr::ParenExpr(expr) => format_paren_expr(fmt, expr),
        Expr::LetExpr(expr) => format_let_expr(fmt, expr),
        Expr::IfExpr(expr) => format_if_expr(fmt, expr, true),
        Expr::BlockExpr(expr) => format_block_or_inline(fmt, expr),
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
        Expr::RecordExpr(expr) => format_record_expr(fmt, expr),
        Expr::MatchExpr(expr) => format_match_expr(fmt, expr),
        Expr::LoopExpr(expr) => format_loop_expr(fmt, expr),
        Expr::WhileExpr(expr) => format_while_expr(fmt, expr),
        Expr::ForExpr(expr) => format_for_expr(fmt, expr),
        Expr::MacroExpr(expr) => format_macro_call(fmt, &expr.macro_call().unwrap()),
        expr => format_unsupported_kind(fmt, expr.syntax()),
    }
}
