//! Chains of postfix expressions (`a.b()?.c`, `a[i]`) and the operators around them

use super::attr::debug_assert_attrs_placed;
use super::expr::{cast_type_parts, format_expr, prefix_op_token, push_ref_operator};
use super::expr_list::format_arg_list;
use super::path::{format_generic_arg_list, format_name_ref};
use crate::formatter::Edge::{Tight, Word};
use crate::formatter::{Builder, Formatter, Frag};
use crate::layout::chain::{ChainElement, ChainLayouts, postfix_chain_layouts};
use crate::layout::head::HeadLayouts;
use crate::layout::list::TrailingSeparator;
use crate::layout::list::{Delimiters, FlatLimit, ListItem, ListLayout, delimited_list};
use crate::syntax_utils::has_attrs;
use ra_ap_syntax::AstNode;
use ra_ap_syntax::ast::{Expr, HasArgList, HasGenericArgs, IndexExpr};

/// The postfix chain of `expr` behind its prefix operators, `&` / `&mut` and casts, if it is one:
/// `a.b()` of `&a.b()`, `!a.b()`, `a.b() as T`, also nested (`&mut *a.b()`); `expr` itself if it is a
/// chain
pub(super) fn operand_chain(expr: &Expr) -> Option<Expr> {
    match expr {
        Expr::RefExpr(e) => operand_chain(&e.expr()?),
        Expr::PrefixExpr(e) => operand_chain(&e.expr()?),
        Expr::CastExpr(e) => operand_chain(&e.expr()?),
        expr if is_postfix_expr(expr) && !has_attrs(expr) => Some(expr.clone()),
        _ => None,
    }
}

/// The layouts of the postfix chain `chain` of `expr` (see `operand_chain`) with the operators of
/// `expr` around them, which then apply to the chain as a whole: `&` in front of a broken chain is
/// followed by its elements at the indentation of `&`, not of the root
pub(super) fn operator_chain_parts(fmt: &Formatter, expr: &Expr, chain: Expr) -> ChainLayouts {
    if *expr == chain {
        return postfix_chain_parts(fmt, chain);
    }
    let (prefix, suffix) = format_operators_around_chain(fmt, expr);
    postfix_chain_parts(fmt, chain)
        .map(|chain| Builder::concat(fmt, [prefix.clone(), chain, suffix.clone()]))
}

/// The prefix operators in front of the chain of `expr` and the casts after it (see
/// `operand_chain`): `&mut *` and `as T` of `&mut *a.b() as T`
fn format_operators_around_chain(fmt: &Formatter, expr: &Expr) -> (Frag, Frag) {
    let mut prefix = Builder::new(fmt);
    let mut casts = Vec::new();
    let mut expr = expr.clone();
    loop {
        expr = match expr {
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
                casts.push(e);
                operand
            }
            _ => break,
        };
    }
    // The innermost cast is the first one after the chain
    let mut suffix = Builder::new(fmt);
    for cast in casts.iter().rev() {
        let (as_token, ty) = cast_type_parts(fmt, cast);
        suffix.push(as_token);
        suffix.push(ty);
    }
    (prefix.into_frag(), suffix.into_frag())
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

/// Splits a postfix chain `a.b()?.c()` into its root (`a`, the return value) and the postfix
/// expressions applied to it, innermost first. An index is not a part of a chain, as in rustfmt (the
/// Style Guide's chain is of field accesses, method calls and `?`): it ends the root, so the root of
/// `a.b[i].c()` is `a.b[i]`, itself a chain followed by an index (see `postfix_chain_parts`); so does
/// a call of the chain's result (`a.b()(x).c()`, see `is_chain_call`). A root ending with an index
/// thus stays on the first line while it fits (`edges.outputs()[0]` / `.take()`), unlike `?`, which
/// stays with its element (`edges` / `.outputs()?` / `.take()`); an index as a suffix of its element
/// was considered and rejected, see PostfixChain in `docs/formatting-taxonomy.md`.
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

/// A chain of fields, method calls, `.await`, `?` and indexes, see `postfix_chain_layouts`
pub(super) fn format_postfix_chain(fmt: &Formatter, expr: Expr) -> Frag {
    postfix_chain_parts(fmt, expr).layouts.all(fmt)
}

/// The layouts of a postfix chain, see `postfix_chain_layouts`. `?` stays with the element (or root)
/// in front of it: `a.b()?`. An index follows the chain in front of it, on the line of its last
/// element, whose `chain_width` it does not count, as in rustfmt: `a.b().c()[0]`, `a` / `.b()` /
/// `.c()[0]`; so do the arguments of a call of the chain's result (`a.b()(x)`, see `is_chain_call`).
/// A root ending with an index or such a call is a chain itself (see `collect_postfix_chain`).
pub(super) fn postfix_chain_parts(fmt: &Formatter, expr: Expr) -> ChainLayouts {
    postfix_chain_parts_then(fmt, expr, &Frag::empty())
}

/// The layouts of the postfix chain `expr` followed by `suffix` on the line of its last element
/// (the indexes after it), see `postfix_chain_parts`
fn postfix_chain_parts_then(fmt: &Formatter, expr: Expr, suffix: &Frag) -> ChainLayouts {
    debug_assert_attrs_placed(fmt, &expr);
    if let Expr::IndexExpr(index_expr) = &expr {
        let index = Builder::concat(fmt, [format_index(fmt, index_expr), suffix.clone()]);
        return ChainLayouts::new(chain_root_layouts(fmt, index_expr.base().unwrap(), &index));
    }
    // A call of the chain's result follows it as an index does
    if let Expr::CallExpr(call) = &expr {
        let args = format_arg_list(fmt, call.arg_list().unwrap());
        let args = Builder::concat(fmt, [args, suffix.clone()]);
        return ChainLayouts::new(chain_root_layouts(fmt, call.expr().unwrap(), &args));
    }
    let mut postfix = Vec::new();
    let root_expr = collect_postfix_chain(expr, &mut postfix);
    // `?` following the root
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

/// The layouts of the root of a postfix chain followed by `suffix` on its last line: a root ending
/// with an index is a chain itself (see `collect_postfix_chain`), another one an expression
fn chain_root_layouts(fmt: &Formatter, expr: Expr, suffix: &Frag) -> HeadLayouts {
    if is_postfix_expr(&expr) && !has_attrs(&expr) {
        postfix_chain_parts_then(fmt, expr, suffix).layouts
    } else {
        let expr = format_expr(fmt, expr);
        HeadLayouts::other(Builder::concat(fmt, [expr, suffix.clone()]))
    }
}

/// Whether the expression is a postfix chain, see `format_postfix_chain`
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

/// Whether the expression calls the result of a postfix chain (`a.b()(x)`, `a.handlers[i](x)`): a
/// chain itself, its arguments following the chain as an index does (see `postfix_chain_parts`)
pub(super) fn is_chain_call(expr: &Expr) -> bool {
    let Expr::CallExpr(call) = expr else {
        return false;
    };
    call.expr()
        .is_some_and(|callee| is_postfix_expr(&callee) && !has_attrs(&callee))
}
