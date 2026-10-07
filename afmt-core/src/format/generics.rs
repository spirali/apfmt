//! Generic parameters, trait bounds and `where` clauses

use super::expr::format_expr;
use super::ty::format_type;
use crate::format::common::{format_lifetime, format_name};
use crate::formatter::Edge::{Comma, Operator, Tight};
use crate::formatter::{Builder, Formatter, Frag};
use crate::layout::chain::{infix_chain, operator_chain};
use crate::layout::list::{
    Delimiters, FlatLimit, ListLayout, TrailingSeparator, delimited_list, keyword_list,
    list_items_cast,
};
use ra_ap_syntax::ast::{
    ConstParam, GenericParam, GenericParamList, HasName, HasTypeBounds, LifetimeParam, TypeBound,
    TypeBoundList, TypeParam, WhereClause, WherePred,
};
use ra_ap_syntax::{AstNode, SyntaxKind};

/// `<'a, T: Clone, const N: usize>`; kept on one line if possible, the Style Guide prefers breaking
/// other parts of an item declaration
pub(super) fn format_generic_param_list(fmt: &Formatter, list: GenericParamList) -> Frag {
    let params = list_items_cast(list.syntax(), |p| format_generic_param(fmt, p));
    let delims = Delimiters::angles_of(list.syntax());
    fmt.keep_together(delimited_list(
        fmt,
        &delims,
        &params,
        ListLayout::tall(FlatLimit::MaxWidth),
    ))
}

fn format_generic_param(fmt: &Formatter, param: GenericParam) -> Frag {
    match param {
        GenericParam::TypeParam(param) => format_type_param(fmt, param),
        GenericParam::LifetimeParam(param) => format_lifetime_param(fmt, param),
        GenericParam::ConstParam(param) => format_const_param(fmt, param),
    }
}

/// `T`, `T: Bound` or `T: Bound = Default`
fn format_type_param(fmt: &Formatter, param: TypeParam) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_name(fmt, param.name().unwrap()));
    push_bounds(fmt, &mut builder, &param);
    if let Some(eq_token) = param.eq_token() {
        builder.push_token(eq_token, Operator);
        builder.push(format_type(fmt, param.default_type().unwrap()));
    }
    builder.into_frag()
}

/// `'a` or `'a: 'b`
fn format_lifetime_param(fmt: &Formatter, param: LifetimeParam) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_lifetime(fmt, param.lifetime().unwrap()));
    push_bounds(fmt, &mut builder, &param);
    builder.into_frag()
}

/// `const N: usize` or `const N: usize = 3`
fn format_const_param(fmt: &Formatter, param: ConstParam) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_keyword(param.const_token().unwrap());
    builder.push(format_name(fmt, param.name().unwrap()));
    builder.push_token(param.colon_token().unwrap(), Comma);
    builder.push(format_type(fmt, param.ty().unwrap()));
    if let Some(eq_token) = param.eq_token() {
        builder.push_token(eq_token, Operator);
        builder.push(format_expr(
            fmt,
            param.default_val().unwrap().expr().unwrap(),
        ));
    }
    builder.into_frag()
}

/// `: A + B` of a node with bounds, if it has them
fn push_bounds(fmt: &Formatter, builder: &mut Builder, node: &impl HasTypeBounds) {
    if let Some(colon) = node.colon_token() {
        builder.push_token(colon, Comma);
        builder.push(format_type_bound_list(fmt, node.type_bound_list().unwrap()));
    }
}

/// `A + B + 'a`: on one line, or broken before every `+`; a type, so kept on
/// one line if possible
pub(super) fn format_type_bound_list(fmt: &Formatter, list: TypeBoundList) -> Frag {
    let (first, rest) = operator_chain(fmt, list.syntax(), SyntaxKind::PLUS, |node| {
        format_type_bound(fmt, TypeBound::cast(node).unwrap())
    })
    .expect("bounds start with a bound");
    fmt.keep_together(infix_chain(fmt, first, &rest))
}

/// `Trait`, `?Sized` or `'a`
fn format_type_bound(fmt: &Formatter, bound: TypeBound) -> Frag {
    let unsupported = bound.for_binder().is_some()
        || bound.use_token().is_some()
        || bound.tilde_token().is_some()
        || bound.const_token().is_some()
        || bound.async_token().is_some()
        || bound.l_brack_token().is_some();
    assert!(!unsupported, "this bound is not supported yet: {bound:?}");
    if let Some(lifetime) = bound.lifetime() {
        return format_lifetime(fmt, lifetime);
    }
    let mut builder = Builder::new(fmt);
    if let Some(token) = bound.question_mark_token() {
        builder.push_token(token, Tight);
    }
    builder.push(format_type(fmt, bound.ty().unwrap()));
    builder.into_frag()
}

/// `where` / `    T: Clone,` / `    'a: 'b,` on its own lines; with `terminated`, the item ends right
/// after it with `;` (`    T: Clone;`), otherwise its body follows on the next line
pub(super) fn format_where_clause(fmt: &Formatter, clause: WhereClause, terminated: bool) -> Frag {
    let predicates = list_items_cast(clause.syntax(), |p| format_where_pred(fmt, p));
    let trailing = if terminated {
        TrailingSeparator::Never
    } else {
        TrailingSeparator::IfVertical
    };
    keyword_list(fmt, clause.where_token().unwrap(), &predicates, trailing)
}

/// `T: Clone + Copy` or `'a: 'b`
fn format_where_pred(fmt: &Formatter, pred: WherePred) -> Frag {
    assert!(
        pred.for_binder().is_none(),
        "`for<...>` is not supported yet: {pred:?}"
    );
    let mut builder = Builder::new(fmt);
    match (pred.lifetime(), pred.ty()) {
        (Some(lifetime), _) => builder.push(format_lifetime(fmt, lifetime)),
        (None, Some(ty)) => builder.push(format_type(fmt, ty)),
        (None, None) => panic!("{pred:?}"),
    }
    push_bounds(fmt, &mut builder, &pred);
    builder.into_frag()
}
