//! Chains of postfix expressions (`a.b()?.c`, `a[i]`) and the operators around them

use super::attr::debug_assert_attrs_placed;
use super::expr::{
    cast_type_parts, format_expr, prefix_op_token, push_ref_operator, unary_operand,
};
use super::expr_list::format_arg_list;
use super::path::{format_generic_arg_list, format_name_ref};
use crate::formatter::Edge::{Close, Open, Tight, Word};
use crate::formatter::{Builder, Formatter, Frag};
use crate::layout::chain::{ChainElement, ChainLayouts, postfix_chain_layouts};
use crate::layout::head::HeadLayouts;
use crate::layout::list::TrailingSeparator;
use crate::layout::list::{Delimiters, FlatLimit, ListItem, ListLayout, delimited_list};
use crate::syntax_utils::has_attrs;
use ra_ap_syntax::AstNode;
use ra_ap_syntax::ast::{CastExpr, Expr, HasArgList, HasGenericArgs, IndexExpr, ParenExpr};

/// The chain behind prefix operators, `&` / `&mut` and casts: `a.b()` of `&mut *a.b() as T`; `expr`
/// itself if it is a chain
pub(super) fn operand_chain(expr: &Expr) -> Option<Expr> {
    operand_chain_through(expr, false)
}

/// Also behind parentheses without comments breaking a line inside them: `a.b()` of `!(a.b())`
pub(super) fn operand_chain_in_parens(expr: &Expr) -> Option<Expr> {
    operand_chain_through(expr, true)
}

fn operand_chain_through(expr: &Expr, parens: bool) -> Option<Expr> {
    match expr {
        Expr::ParenExpr(e)
            if parens
                && !has_attrs(&e.expr()?)
                && Delimiters::try_parens_of(e.syntax())
                    .is_some_and(|d| !d.comments_break_body()) =>
        {
            operand_chain_through(&e.expr()?, parens)
        }
        expr if is_postfix_expr(expr) && !has_attrs(expr) => Some(expr.clone()),
        expr => operand_chain_through(&unary_operand(expr)?, parens),
    }
}

/// The chain layouts with the operators of `expr` around them, applying to the chain as a whole
pub(super) fn operator_chain_parts(fmt: &Formatter, expr: &Expr, chain: Expr) -> ChainLayouts {
    if *expr == chain {
        return postfix_chain_parts(fmt, chain);
    }
    let (prefix, suffix) = format_operators_around_chain(fmt, expr);
    postfix_chain_parts(fmt, chain)
        .map(|chain| Builder::concat(fmt, [prefix.clone(), chain, suffix.clone()]))
}

/// E.g. `&mut *(` and `as T)` of `&mut *(a.b() as T)`: what `operand_chain_in_parens` strips
fn format_operators_around_chain(fmt: &Formatter, expr: &Expr) -> (Frag, Frag) {
    let mut prefix = Builder::new(fmt);
    let mut suffixes = Vec::new();
    let mut expr = expr.clone();
    loop {
        expr = match expr {
            Expr::ParenExpr(e) => {
                prefix.push_token(e.l_paren_token().unwrap(), Open);
                let inner = e.expr().unwrap();
                suffixes.push(ChainSuffix::Paren(e));
                inner
            }
            Expr::RefExpr(e) => {
                push_ref_operator(&mut prefix, &e);
                e.expr().unwrap()
            }
            Expr::PrefixExpr(e) => {
                prefix.push_token(prefix_op_token(&e), Tight);
                e.expr().unwrap()
            }
            Expr::CastExpr(e) => {
                let operand = e.expr().unwrap();
                suffixes.push(ChainSuffix::Cast(e));
                operand
            }
            _ => break,
        };
    }
    // The innermost suffix is the first one after the chain
    let mut suffix = Builder::new(fmt);
    for item in suffixes.iter().rev() {
        match item {
            ChainSuffix::Cast(cast) => {
                let (as_token, ty) = cast_type_parts(fmt, cast);
                suffix.push(as_token);
                suffix.push(ty);
            }
            ChainSuffix::Paren(paren) => suffix.push_token(paren.r_paren_token().unwrap(), Close),
        }
    }
    (prefix.into_frag(), suffix.into_frag())
}

enum ChainSuffix {
    Cast(CastExpr),
    Paren(ParenExpr),
}

/// `[index]`, broken only when it does not fit, between the brackets (Style Guide): `target[` /
/// `    index` / `]`
fn format_index(fmt: &Formatter, expr: &IndexExpr) -> Frag {
    let index = ListItem::new(format_expr(fmt, expr.index().unwrap()));
    let layout =
        ListLayout::tall(FlatLimit::MaxWidth).trailing_separator(TrailingSeparator::Omitted);
    let delims = Delimiters::brackets_of(expr.syntax());
    fmt.keep_together(delimited_list(fmt, &delims, &[index], layout))
}

/// Splits `a.b()?.c()` into the root and the postfix expressions, innermost first. An index or a
/// chain call ends the root, as in rustfmt (see PostfixChain in `docs/formatting-taxonomy.md`).
fn collect_postfix_chain(expr: Expr, postfix: &mut Vec<Expr>) -> Expr {
    let inner = match &expr {
        Expr::MethodCallExpr(e) => e.receiver(),
        Expr::FieldExpr(e) => e.expr(),
        Expr::AwaitExpr(e) => e.expr(),
        Expr::TryExpr(e) => e.expr(),
        _ => return expr,
    };
    let root = collect_postfix_chain(inner.unwrap(), postfix);
    postfix.push(expr);
    root
}

pub(super) fn format_postfix_chain(fmt: &Formatter, expr: Expr) -> Frag {
    postfix_chain_parts(fmt, expr).layouts.all(fmt)
}

/// `?` stays with the element in front of it. An index or a chain call follows the last element on
/// its line, not counted in `chain_width`, as in rustfmt: `a` / `.b()` / `.c()[0]`.
pub(super) fn postfix_chain_parts(fmt: &Formatter, expr: Expr) -> ChainLayouts {
    postfix_chain_parts_then(fmt, expr, &Frag::empty())
}

/// `suffix` (the indexes after it) follows on the line of its last element
fn postfix_chain_parts_then(fmt: &Formatter, expr: Expr, suffix: &Frag) -> ChainLayouts {
    debug_assert_attrs_placed(fmt, &expr);
    if let Expr::IndexExpr(index_expr) = &expr {
        let index = Builder::concat(fmt, [format_index(fmt, index_expr), suffix.clone()]);
        return ChainLayouts::new(chain_root_layouts(fmt, index_expr.base().unwrap(), &index));
    }
    if let Expr::CallExpr(call) = &expr {
        let args = format_arg_list(fmt, call.arg_list().unwrap());
        let args = Builder::concat(fmt, [args, suffix.clone()]);
        return ChainLayouts::new(chain_root_layouts(fmt, call.expr().unwrap(), &args));
    }
    let mut postfix = Vec::new();
    let root_expr = collect_postfix_chain(expr, &mut postfix);
    let mut root = Builder::new(fmt);
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
    let root_suffix = root.into_frag();
    if elements.is_empty() {
        // `a[i]?`: the root is followed by the `?` and the suffix on its last line
        let suffix = Builder::concat(fmt, [root_suffix, suffix.clone()]);
        return ChainLayouts::new(chain_root_layouts(fmt, root_expr, &suffix));
    }
    let root = chain_root_layouts(fmt, root_expr, &root_suffix);
    postfix_chain_layouts(fmt, root, &elements, suffix)
}

/// A root ending with an index is a chain itself, another one an expression
fn chain_root_layouts(fmt: &Formatter, expr: Expr, suffix: &Frag) -> HeadLayouts {
    if is_postfix_expr(&expr) && !has_attrs(&expr) {
        postfix_chain_parts_then(fmt, expr, suffix).layouts
    } else {
        let expr = format_expr(fmt, expr);
        HeadLayouts::other(Builder::concat(fmt, [expr, suffix.clone()]))
    }
}

pub(super) fn is_postfix_expr(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::MethodCallExpr(_)
            | Expr::FieldExpr(_)
            | Expr::AwaitExpr(_)
            | Expr::TryExpr(_)
            | Expr::IndexExpr(_)
    ) || is_chain_call(expr)
}

/// `a.b()(x)`, `a.handlers[i](x)`: a chain itself, its arguments following it as an index does
pub(super) fn is_chain_call(expr: &Expr) -> bool {
    let Expr::CallExpr(call) = expr else {
        return false;
    };
    call.expr()
        .is_some_and(|callee| is_postfix_expr(&callee) && !has_attrs(&callee))
}
