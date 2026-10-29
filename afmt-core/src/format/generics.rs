//! Generic parameters, trait bounds and `where` clauses

use super::expr::format_expr;
use super::path::format_name_ref;
use super::ty::format_type;
use crate::format::common::{format_lifetime, format_name};
use crate::formatter::Edge::{Comma, Keyword, Operator, Tight, Word};
use crate::formatter::{Builder, Formatter, Frag};
use crate::layout::chain::{infix_chain, operator_chain, unindented_infix_chain};
use crate::layout::list::{
    Delimiters, FlatLimit, ListItem, ListLayout, TrailingSeparator, delimited_list, keyword_list,
    list_items_cast,
};
use ra_ap_syntax::ast::{
    ConstParam, ForBinder, GenericParam, GenericParamList, HasName, HasTypeBounds, LifetimeParam,
    TypeBound, TypeBoundList, TypeParam, UseBoundGenericArg, UseBoundGenericArgs, WhereClause,
    WherePred,
};
use ra_ap_syntax::{AstNode, SyntaxKind, SyntaxNode};
use std::convert::identity;

/// `<'a, T: Clone, const N: usize>`, kept on one line if possible (Style Guide)
pub(super) fn format_generic_param_list(fmt: &Formatter, list: GenericParamList) -> Frag {
    let params = list_items_cast(list.syntax(), |p| format_generic_param(fmt, p));
    format_angle_list(fmt, list.syntax(), &params)
}

/// Generic parameters or arguments: on one line, or one per line; kept on one line if possible
/// ("avoid breaking lines in types where possible", Style Guide)
pub(super) fn format_angle_list(fmt: &Formatter, node: &SyntaxNode, items: &[ListItem]) -> Frag {
    let delims = Delimiters::angles_of(node);
    fmt.keep_together(delimited_list(
        fmt,
        &delims,
        items,
        ListLayout::tall(FlatLimit::MaxWidth),
    ))
}

/// `for<'a> `, broken only as a last resort: the bound or type after it breaks first
pub(super) fn format_for_binder(fmt: &Formatter, binder: ForBinder) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_token(binder.for_token().unwrap(), Word);
    let params = format_generic_param_list(fmt, binder.generic_param_list().unwrap());
    builder.push(fmt.keep_together(params));
    fmt.followed_by_space(builder.into_frag())
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
    push_bounds(fmt, &mut builder, &param, identity);
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
    push_bounds(fmt, &mut builder, &param, identity);
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

/// `: A + B`, or only `:` if the list is empty (`T:`); `place` places the bounds after `:`
fn push_bounds(
    fmt: &Formatter,
    builder: &mut Builder,
    node: &impl HasTypeBounds,
    place: impl FnOnce(Frag) -> Frag,
) {
    if let Some(colon) = node.colon_token() {
        builder.push_token(colon, Comma);
        let list = node.type_bound_list();
        if let Some(list) = list.filter(|list| list.bounds().next().is_some()) {
            builder.push(place(format_type_bound_list(fmt, list)));
        }
    }
}

/// `A + B + 'a`: on one line, or broken before every `+`; kept on one line if possible
pub(super) fn format_type_bound_list(fmt: &Formatter, list: TypeBoundList) -> Frag {
    let (first, rest) = type_bound_operands(fmt, &list);
    fmt.keep_together(infix_chain(fmt, first, &rest))
}

/// As `format_type_bound_list`, but `+` at the indentation of the first bound (supertraits: `trait
/// A:` / `    B` / `    + C`)
pub(super) fn format_unindented_type_bound_list(fmt: &Formatter, list: TypeBoundList) -> Frag {
    let (first, rest) = type_bound_operands(fmt, &list);
    fmt.keep_together(unindented_infix_chain(fmt, first, &rest))
}

fn type_bound_operands(fmt: &Formatter, list: &TypeBoundList) -> (Frag, Vec<(Frag, Frag)>) {
    operator_chain(fmt, list.syntax(), SyntaxKind::PLUS, |node| {
        format_type_bound(fmt, TypeBound::cast(node).unwrap())
    })
}

/// `'a`, `use<'a, T>` or a trait bound, in its source parentheses: `(FnOnce() -> R) + Send`
fn format_type_bound(fmt: &Formatter, bound: TypeBound) -> Frag {
    let mut builder = Builder::new(fmt);
    if let Some(lifetime) = bound.lifetime() {
        builder.push(format_lifetime(fmt, lifetime));
    } else if let Some(args) = bound.use_bound_generic_args() {
        builder.push_token(bound.use_token().unwrap(), Word);
        builder.push(format_use_bound_generic_args(fmt, args));
    } else {
        push_trait_bound(fmt, &mut builder, &bound);
    }
    let inner = builder.into_frag();
    match Delimiters::try_parens_of(bound.syntax()) {
        Some(parens) => parens.enclose(fmt, inner),
        None => inner,
    }
}

/// `Trait`, `?Sized`, `for<'a> Fn(&'a T)`, `~const Trait`, `[const] Trait`, `async Fn()`
fn push_trait_bound(fmt: &Formatter, builder: &mut Builder, bound: &TypeBound) {
    if let Some(binder) = bound.for_binder() {
        builder.push(format_for_binder(fmt, binder));
    }
    if let Some(l_brack) = bound.l_brack_token() {
        builder.push_token(l_brack, Tight);
        builder.push_token(bound.const_token().unwrap(), Tight);
        builder.push_token(bound.r_brack_token().unwrap(), Keyword);
    } else {
        if let Some(tilde) = bound.tilde_token() {
            builder.push_token(tilde, Tight);
        }
        if let Some(token) = bound.const_token() {
            builder.push_keyword(token);
        }
    }
    if let Some(token) = bound.async_token() {
        builder.push_keyword(token);
    }
    if let Some(token) = bound.question_mark_token() {
        builder.push_token(token, Tight);
    }
    builder.push(format_type(fmt, bound.ty().unwrap()));
}

/// `<'a, T>` of a `use` bound
fn format_use_bound_generic_args(fmt: &Formatter, args: UseBoundGenericArgs) -> Frag {
    let items = list_items_cast(args.syntax(), |arg| match arg {
        UseBoundGenericArg::Lifetime(lifetime) => format_lifetime(fmt, lifetime),
        UseBoundGenericArg::NameRef(name_ref) => format_name_ref(fmt, name_ref),
    });
    format_angle_list(fmt, args.syntax(), &items)
}

/// `where` / `    T: Clone,` on its own lines; with `terminated`, the item ends with `;` right
/// after
pub(super) fn format_where_clause(fmt: &Formatter, clause: WhereClause, terminated: bool) -> Frag {
    let predicates = list_items_cast(clause.syntax(), |p| format_where_pred(fmt, p));
    let trailing = if terminated {
        TrailingSeparator::Never
    } else {
        TrailingSeparator::IfVertical
    };
    keyword_list(fmt, clause.where_token().unwrap(), &predicates, trailing)
}

/// `T: Clone + Copy`, `for<'a> &'a T: Trait` or `'a: 'b`
fn format_where_pred(fmt: &Formatter, pred: WherePred) -> Frag {
    let mut builder = Builder::new(fmt);
    if let Some(binder) = pred.for_binder() {
        builder.push(format_for_binder(fmt, binder));
    }
    match (pred.lifetime(), pred.ty()) {
        (Some(lifetime), _) => builder.push(format_lifetime(fmt, lifetime)),
        (None, Some(ty)) => builder.push(format_type(fmt, ty)),
        (None, None) => panic!("{pred:?}"),
    }
    // Bounds that do not fit move to the next line (as rustfmt): `F:` / `    Fn(&'a u8) -> u8,`
    push_bounds(fmt, &mut builder, &pred, |bounds| {
        fmt.same_or_indented_line(bounds)
    });
    builder.into_frag()
}
