//! Expressions

use super::attr::{debug_assert_attrs_placed, format_skippable_with_attrs, format_with_attrs};
use super::chain::{
    format_postfix_chain, is_postfix_expr, operand_chain_in_parens, operator_chain_parts,
    postfix_chain_parts,
};
use super::closure::format_closure_expr;
use super::control::{format_for_expr, format_loop_expr, format_match_expr, format_while_expr};
use super::expr_list::{format_arg_list, format_array_expr, format_tuple_expr, is_literal_or_name};
use super::macros::format_macro_call;
use super::pat::pat_layouts;
use super::path::{format_name_ref, format_path};
use super::stmt::{
    format_block_expr, format_block_expr_with_tail, format_block_tail, push_block_prefix,
};
use super::ty::format_type;
use super::{format_unsupported_kind, format_unsupported_with_attrs};
use crate::comments::{comments_break_line_after, is_code_kind, is_comment_kind};
use crate::doc::{CLOSE_BRACE_DOC, OPEN_BRACE_DOC};
use crate::format::common::format_lifetime;
use crate::formatter::Edge::{BraceClose, BraceOpen, Comma, Operator, Tight, Word};
use crate::formatter::{Builder, Formatter, Frag};
use crate::layout::chain::{broken_infix_chain, infix_chain, short_first_operand};
use crate::layout::head::{HeadLayouts, pattern_head};
use crate::layout::list::TrailingSeparator;
use crate::layout::list::{
    Delimiters, FlatLimit, ListItem, ListLayout, delimited_list, list_items_cast,
};
use crate::syntax_utils::{has_attrs, is_brace_macro_expr};
use ra_ap_syntax::ast::{
    BinExpr, BinaryOp, BlockExpr, BreakExpr, CallExpr, CastExpr, ContinueExpr, ElseBranch, Expr,
    HasArgList, HasAttrs, IfExpr, LetExpr, Literal, LogicOp, ParenExpr, PathExpr, PrefixExpr,
    RangeExpr, RangeItem, RecordExpr, RecordExprField, RefExpr, ReturnExpr, StmtList,
};
use ra_ap_syntax::{AstNode, SyntaxNode, SyntaxToken};

fn format_path_expr(fmt: &Formatter, path_expr: PathExpr) -> Frag {
    format_path(fmt, path_expr.path().unwrap())
}

/// `callee(args)`: a broken callee costs a split, so the arguments break first (Style Guide:
/// "Prefer not to break a line in the callee expression")
fn format_call_expr(fmt: &Formatter, call: CallExpr) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(fmt.keep_together(format_expr(fmt, call.expr().unwrap())));
    builder.push(format_arg_list(fmt, call.arg_list().unwrap()));
    builder.into_frag()
}

fn format_literal(fmt: &Formatter, literal: Literal) -> Frag {
    fmt.token(literal.token(), Word)
}

/// Splits `a && b && c` into the first operand (returned) and the (operator, operand) pairs
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
    let chain = infix_chain(fmt, first.clone(), &rest);
    let [(op, operand)] = rest.as_slice() else {
        return chain;
    };
    // As rustfmt: the right operand stays on the line when `first op ` is at most `tab_spaces` wide
    let op_width = fmt.min_flat_width(op).unwrap_or(0) + 2;
    let max_width = fmt.config().tab_spaces.saturating_sub(op_width);
    let short_first = short_first_operand(fmt, &first, op, operand, max_width);
    fmt.alt_opt(chain, short_first)
}

fn bin_chain_operands(fmt: &Formatter, expr: BinExpr) -> (Frag, Vec<(Frag, Frag)>) {
    let mut rest = Vec::new();
    let first = format_expr(fmt, collect_bin_chain(expr, &mut rest));
    let rest = rest
        .into_iter()
        .map(|(op, e)| (fmt.token(op, Operator), format_expr(fmt, e)))
        .collect();
    (first, rest)
}

/// `lhs = rhs`, `lhs += rhs`, ...: broken after the operator, the left side only when nothing else
/// helps, like `let`
fn format_assignment(fmt: &Formatter, expr: BinExpr) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(fmt.keep_together(format_expr(fmt, expr.lhs().unwrap())));
    builder.push_token(expr.op_token().unwrap(), Operator);
    builder.push(format_rhs(fmt, expr.rhs().unwrap()));
    builder.into_frag()
}

/// The right side of `let` or an assignment: on the next line when that saves at least two lines;
/// a chain broken only in its last arguments already when it saves one, as in rustfmt
pub(super) fn format_rhs(fmt: &Formatter, expr: Expr) -> Frag {
    if !has_attrs(&expr) && is_postfix_expr(&expr) {
        return match postfix_chain_parts(fmt, expr).layouts {
            HeadLayouts {
                hanging: Some(hanging),
                other: Some(other),
            } => fmt.same_or_indented_line_saving_lines_preferring(hanging, other),
            layouts => fmt.same_or_indented_line_saving_lines(layouts.all(fmt)),
        };
    }
    fmt.same_or_indented_line_saving_lines(format_expr(fmt, expr))
}

/// Its first token child; not `PrefixExpr::op_token`, which is an attribute in `#[cfg(x)] -1`
pub(super) fn prefix_op_token(expr: &PrefixExpr) -> SyntaxToken {
    expr.syntax()
        .children_with_tokens()
        .find_map(|e| e.into_token().filter(|t| is_code_kind(t.kind())))
        .unwrap()
}

fn format_prefix_expr(fmt: &Formatter, expr: PrefixExpr) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_token(prefix_op_token(&expr), Tight);
    builder.push(format_expr(fmt, expr.expr().unwrap()));
    builder.into_frag()
}

/// The operator of `&x`, `&mut x`, `&raw const x`
pub(super) fn push_ref_operator(builder: &mut Builder, expr: &RefExpr) {
    builder.push_token(expr.amp_token().unwrap(), Tight);
    for token in [expr.raw_token(), expr.const_token(), expr.mut_token()]
        .into_iter()
        .flatten()
    {
        builder.push_keyword(token);
    }
}

pub(super) fn cast_type_parts(fmt: &Formatter, expr: &CastExpr) -> (Frag, Frag) {
    let as_token = fmt.token(expr.as_token().unwrap(), Operator);
    (as_token, format_type(fmt, expr.ty().unwrap()))
}

fn format_paren_expr(fmt: &Formatter, expr: ParenExpr) -> Frag {
    let inner = format_expr(fmt, expr.expr().unwrap());
    Delimiters::parens_of(expr.syntax()).enclose(fmt, inner)
}

/// `&x`, `&mut x`, `&raw const x`
fn format_ref_expr(fmt: &Formatter, expr: RefExpr) -> Frag {
    let mut builder = Builder::new(fmt);
    push_ref_operator(&mut builder, &expr);
    builder.push(format_expr(fmt, expr.expr().unwrap()));
    builder.into_frag()
}

/// `expr as A as B`: like a binary operator, or broken only before the first `as`; the types may
/// also follow a broken operand on its last line (`(a` / `    + b) as u64`), as in rustfmt
fn format_cast_expr(fmt: &Formatter, expr: CastExpr) -> Frag {
    let mut casts = vec![expr];
    while let Some(Expr::CastExpr(inner)) = casts.last().unwrap().expr() {
        casts.push(inner);
    }
    let first = format_expr(fmt, casts.last().unwrap().expr().unwrap());
    let rest = casts
        .iter()
        .rev()
        .map(|cast| cast_type_parts(fmt, cast))
        .collect::<Vec<_>>();
    let chain = infix_chain(fmt, first.clone(), &rest);
    let types = rest
        .iter()
        .flat_map(|(as_token, ty)| [as_token.clone(), ty.clone()]);
    let types = fmt.flat(Builder::concat(fmt, types));
    let after_operand = Builder::concat(fmt, [fmt.multi_line(first.clone()), types.clone()]);
    let chain = fmt.alt_frag(chain, after_operand);
    if rest.len() < 2 {
        return chain;
    }
    let mut types_line = Builder::new(fmt);
    types_line.push(first);
    types_line.push(fmt.indented_line(types));
    fmt.alt_frag(chain, types_line.into_frag())
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

/// `a..b` as a binary operator, broken before `..` (as rustfmt): `a` / `    ..b`; after a start of
/// at most `tab_spaces` columns, the end may break on its line (`0..f(` / `    x,` / `)`, as rustfmt)
fn format_range_expr(fmt: &Formatter, expr: RangeExpr) -> Frag {
    let (Some(start), Some(end)) = (expr.start(), expr.end()) else {
        return format_range(fmt, &expr, |e| format_expr(fmt, e));
    };
    let start = format_expr(fmt, start);
    let op = fmt.token(expr.op_token().unwrap(), Tight);
    let end = format_expr(fmt, end);
    let chain = infix_chain(fmt, start.clone(), &[(op.clone(), end.clone())]);
    let short_start = short_first_operand(fmt, &start, &op, &end, fmt.config().tab_spaces);
    fmt.alt_opt(chain, short_start)
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

/// `let pat = expr` in a condition, with the layouts of the head of `if let`
fn format_let_expr(fmt: &Formatter, expr: LetExpr) -> Frag {
    let_head_layouts(fmt, &expr).all(fmt)
}

/// A broken `a && b` or `a..b`, also in parentheses or under a prefix operator or a cast (`!(a` / `    && b)`),
/// ends with an indented continuation line: `{` then goes on its own line (chains have their own layouts)
fn is_hanging_expr(expr: &Expr) -> bool {
    match expr {
        Expr::BinExpr(_) | Expr::RangeExpr(_) => true,
        Expr::ParenExpr(e) => e.expr().is_some_and(|e| is_hanging_expr(&e)),
        Expr::PrefixExpr(e) => e.expr().is_some_and(|e| is_hanging_expr(&e)),
        Expr::RefExpr(e) => e.expr().is_some_and(|e| is_hanging_expr(&e)),
        Expr::CastExpr(e) => e.expr().is_some_and(|e| is_hanging_expr(&e)),
        _ => false,
    }
}

/// `keyword head {` of a control-flow expression. A broken head costs a split, as `{` on its own
/// line does (Style Guide: break a control line only when it does not fit).
pub(super) fn format_head_with_block(
    fmt: &Formatter,
    keyword: SyntaxToken,
    head: &HeadLayouts,
    block: Frag,
) -> Frag {
    let comment_after_keyword = comments_break_line_after(&keyword);
    let mut builder = Builder::new(fmt);
    builder.push_keyword(keyword);
    let head = head.clone().map(|head| fmt.keep_together(head));
    if comment_after_keyword {
        push_head_after_comments(fmt, &mut builder, head, block);
    } else {
        builder.push(format_head_followed_by(fmt, &head, block));
    }
    builder.into_frag()
}

/// `head` after comments ending the line of the cursor, then `block` on its own line: the head
/// hangs (Style Guide): `if` / `    // c` / `    cond` / `{`
pub(super) fn push_head_after_comments(
    fmt: &Formatter,
    builder: &mut Builder,
    head: HeadLayouts,
    block: Frag,
) {
    builder.push(head.all(fmt));
    builder.push(fmt.on_next_line(block));
}

/// `head` followed by `block`; after a broken head with a hanging indent, `{` goes on its own line,
/// except after a last line of only closing delimiters (`if f(` / `    a,` / `) {`)
pub(super) fn format_head_followed_by(fmt: &Formatter, head: &HeadLayouts, block: Frag) -> Frag {
    let next_line = |head: Frag| {
        let mut builder = Builder::new(fmt);
        builder.push(head);
        builder.push(fmt.on_next_line(block.clone()));
        builder.into_frag()
    };
    let closing = (head.hanging.clone()).and_then(|hanging| fmt.closing_last_line(hanging));
    let same_line = (fmt.alt_some(head.other.clone(), closing))
        .map(|head| Builder::concat(fmt, [head, block.clone()]));
    // `{` on its own line costs a split. Not after `closing`: those layouts are in
    // `next_line(hanging)`.
    let broken = (head.other.clone()).map(|other| fmt.dispreferred(next_line(other)));
    let mut layouts = fmt.alt_some(same_line, broken);
    if let Some(hanging) = &head.hanging {
        layouts = Some(fmt.alt_opt(next_line(hanging.clone()), layouts));
    }
    layouts.unwrap()
}

/// No label or modifier in front of `{` (`unsafe {` is an expression)
pub(super) fn plain_stmt_list(block: &BlockExpr) -> Option<StmtList> {
    let stmt_list = block.stmt_list()?;
    (block.syntax().first_token() == stmt_list.l_curly_token()).then_some(stmt_list)
}

/// Nested plain blocks each holding just the next one, `{ { expr } }`: the outer braces are removed
/// (as in rustfmt)
pub(super) struct NestedPlainBlocks {
    pub inner: BlockExpr,
    /// The `}` of the outermost block
    close: SyntaxToken,
    nested: bool,
}

impl NestedPlainBlocks {
    /// `block` must be a plain one
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

    pub fn skip_outer_braces(&self, fmt: &Formatter, frag: Frag) -> Frag {
        if self.nested {
            Builder::skip_to_token_after(fmt, frag, self.close.clone())
        } else {
            frag
        }
    }

    pub fn skip_braces(&self, fmt: &Formatter, frag: Frag) -> Frag {
        Builder::skip_to_token_after(fmt, frag, self.close.clone())
    }
}

/// The expression of a block that can be `{ expr }`: no statements, comments or attributes; not a
/// `try` block, nor a macro call with braces, which is a statement there
pub(super) fn single_line_block_expr(block: &BlockExpr) -> Option<Expr> {
    let stmt_list = block.stmt_list()?;
    let has_comment = stmt_list
        .syntax()
        .children_with_tokens()
        .any(|e| is_comment_kind(e.kind()));
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

/// The caller restricts it to a single line
pub(super) fn format_braced_block(fmt: &Formatter, block: &BlockExpr, inner: Frag) -> Frag {
    Delimiters::braces_of(block.stmt_list().unwrap().syntax()).enclose(fmt, inner)
}

/// A single `else` and blocks containing just an expression; a syntactic check only
fn single_line_if_exprs(expr: &IfExpr) -> Option<(Expr, Expr)> {
    let Some(ElseBranch::Block(else_block)) = expr.else_branch() else {
        return None;
    };
    Some((
        single_line_block_expr(&expr.then_branch()?)?,
        single_line_block_expr(&else_block)?,
    ))
}

/// `if a { b } else { c }` on a single line; only in an expression context
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
        // Own-line comments in front of `else` keep the indentation of `}`
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
        block_exprs.and_then(|(then_expr, else_expr)| {
            let condition = head.other.clone()?;
            format_if_single_line(fmt, &expr, condition, then_expr, else_expr)
        }),
    )
}

/// Broken, or `unsafe { x }` on a single line; a block in expression position (Style Guide
/// "Blocks")
pub(super) fn format_block_or_inline(fmt: &Formatter, block: BlockExpr) -> Frag {
    debug_assert_attrs_placed(fmt, &block);
    let Some(tail) = single_line_block_expr(&block) else {
        return format_block_expr(fmt, block);
    };
    let tail = format_expr(fmt, tail);
    let (inline, broken) = inline_and_broken_block(fmt, block, tail);
    fmt.alt_frag(inline, broken)
}

/// `tail` is its formatted expression (see `single_line_block_expr`)
pub(super) fn inline_and_broken_block(
    fmt: &Formatter,
    block: BlockExpr,
    tail: Frag,
) -> (Frag, Frag) {
    let mut inline = Builder::new(fmt);
    push_block_prefix(fmt, &mut inline, &block);
    inline.push(format_braced_block(fmt, &block, tail.clone()));
    let broken = format_block_expr_with_tail(fmt, block, Some(tail));
    (fmt.flat(inline.into_frag()), broken)
}

/// `{` / `    inner` / `}` with braces that are not in the source, e.g. around a closure body
pub(super) fn format_added_braces(fmt: &Formatter, node: &SyntaxNode, inner: Frag) -> Frag {
    let mut braced = Builder::new(fmt);
    braced.push_generated(OPEN_BRACE_DOC, BraceOpen);
    let mut line = Builder::new(fmt);
    line.push(format_block_tail(fmt, node, inner));
    line.push_newline();
    braced.push(fmt.body_after_opener(false, line.into_frag()));
    braced.push_generated(CLOSE_BRACE_DOC, BraceClose);
    braced.into_frag()
}

/// `Path { a, b: c, ..base }`: on a single line within `struct_lit_width`, otherwise one field per
/// line; `..base` has no comma
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
    // Blank lines between the fields are kept, as in a struct declaration (as rustfmt)
    let layout = ListLayout::tall(FlatLimit::Within(fmt.config().struct_lit_width))
        .trailing_separator(trailing)
        .keep_blank_lines();
    let delims = Delimiters::braces_of(list.syntax());
    builder.push(delimited_list(fmt, &delims, &fields, layout));
    builder.into_frag()
}

/// `name: value`, the value on the next line only if it overflows (RhsBreak, as rustfmt), or `name`
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

/// `let pat = expr` of `if let`, `while let` or a let chain; the expression moves after `=` when it
/// fits there or saves a line (Style Guide), unlike a `let` statement.
fn let_head_layouts(fmt: &Formatter, expr: &LetExpr) -> HeadLayouts {
    let pat = pat_layouts(fmt, expr.pat().unwrap());
    let all_pat = pat.clone().all(fmt);
    let let_token = fmt.keyword(expr.let_token().unwrap());
    let eq = fmt.token(expr.eq_token().unwrap(), Operator);
    let with_pat =
        |pat: Frag, rhs: Frag| Builder::concat(fmt, [let_token.clone(), pat, eq.clone(), rhs]);
    let rhs = format_head_expr(fmt, expr.expr().unwrap()).map(|frag| fmt.keep_together(frag));
    let HeadLayouts { hanging, other } = pattern_head(fmt, &pat, &all_pat, &rhs, with_pat);
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

/// A let chain that the Style Guide breaks before every `&&`: not just a simple operand and a `let`
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
    if has_attrs(&expr) {
        return HeadLayouts::other(format_expr(fmt, expr));
    }
    if is_postfix_expr(&expr) {
        return postfix_chain_parts(fmt, expr).layouts;
    }
    if let Expr::LetExpr(let_expr) = &expr {
        return let_head_layouts(fmt, let_expr);
    }
    // `!a.b()`, `&a.b()`, `a.b() as T`, `(a.b())`: hangs as the chain does
    if let Some(chain) = operand_chain_in_parens(&expr) {
        return operator_chain_parts(fmt, &expr, chain).layouts;
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
            // Broken only when the head does not fit: `if a && b` / `{` comes first
            hanging: Some(fmt.multi_line(frag.clone())),
            other: Some(fmt.flat(frag)),
        }
    } else {
        HeadLayouts::other(frag)
    }
}

/// With outer attributes, it is copied unchanged (see `format_expr_at`)
pub(super) fn format_expr(fmt: &Formatter, expr: Expr) -> Frag {
    if has_attrs(&expr) {
        return format_unsupported_with_attrs(fmt, expr.syntax(), "an attribute on an expression");
    }
    format_expr_without_attrs(fmt, expr)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ExprPosition {
    Statement,
    Tail,
    ArmBody,
}

/// The attributes of a statement or a tail are on their own lines; elsewhere copied unchanged. A
/// standalone `if` or a statement block (but `unsafe`) is never on a single line (Style Guide).
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

/// The caller places the outer attributes
pub(super) fn format_expr_without_attrs(fmt: &Formatter, expr: Expr) -> Frag {
    debug_assert_attrs_placed(fmt, &expr);
    match expr {
        Expr::Literal(expr) => format_literal(fmt, expr),
        // Also a call of a chain's result, see `is_chain_call`
        expr if is_postfix_expr(&expr) => format_postfix_chain(fmt, expr),
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
