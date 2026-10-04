//! Paths and generic arguments

use crate::formatter::{Builder, Formatter, Frag};
use crate::formatter::Edge::{Operator, Tight, Word};
use crate::layout::list::{flat_list, list_items_cast};
use ra_ap_syntax::AstNode;
use ra_ap_syntax::ast::{
    AssocTypeArg, GenericArg, GenericArgList, HasGenericArgs, HasTypeBounds, NameRef,
    ParenthesizedArgList, Path, PathSegment, Type, TypeAnchor, TypeArg,
};
use crate::format::common::format_parenthesized;
use super::expr::format_expr;
use super::ty::{format_ret_type, format_type};

/// `a::b::c`; the path is left-recursive: `(a::b)::c`
pub(super) fn format_path(fmt: &Formatter, path: Path) -> Frag {
    let mut builder = Builder::new(fmt);
    if let Some(qualifier) = path.qualifier() {
        builder.push(format_path(fmt, qualifier));
        builder.push_token(path.coloncolon_token().unwrap(), Tight);
    }
    builder.push(format_path_segment(fmt, path.segment().unwrap()));
    builder.to_frag()
}

/// `name`, `name<args>`, `name::<args>`, `Fn(args) -> ret` or `<T as Trait>`; with a leading `::` in `::std`
fn format_path_segment(fmt: &Formatter, segment: PathSegment) -> Frag {
    let mut builder = Builder::new(fmt);
    if let Some(token) = segment.coloncolon_token() {
        builder.push_token(token, Tight);
    }
    if let Some(name_ref) = segment.name_ref() {
        builder.push(format_name_ref(fmt, name_ref));
    }
    if let Some(anchor) = segment.type_anchor() {
        builder.push(format_type_anchor(fmt, anchor));
    }
    if let Some(args) = segment.generic_arg_list() {
        builder.push(format_generic_arg_list(fmt, args));
    }
    if let Some(args) = segment.parenthesized_arg_list() {
        builder.push(format_parenthesized_arg_list(fmt, args));
    }
    if let Some(ret_type) = segment.ret_type() {
        builder.push(format_ret_type(fmt, ret_type));
    }
    if let Some(x) = segment.return_type_syntax() {
        panic!("{x:?}");
    }
    builder.to_frag()
}

/// An identifier or one of `self`, `super`, `crate`, `Self`
fn format_name_ref(fmt: &Formatter, name_ref: NameRef) -> Frag {
    fmt.token(name_ref.syntax().first_token().unwrap(), Word)
}

/// `<T>` or `<T as Trait>` in `<T as Trait>::f`
fn format_type_anchor(fmt: &Formatter, anchor: TypeAnchor) -> Frag {
    let mut inner = Builder::new(fmt);
    inner.push(format_type(fmt, anchor.ty().unwrap()));
    if let Some(as_token) = anchor.as_token() {
        inner.push_token(as_token, Word);
        // `path_type()` returns the first path type, which is `T` in `<T as Trait>` (rust-analyzer#15778)
        let trait_ty = anchor
            .syntax()
            .children()
            .filter_map(Type::cast)
            .nth(1)
            .unwrap();
        inner.push(format_type(fmt, trait_ty));
    }
    format_parenthesized(
        fmt,
        anchor.l_angle_token().unwrap(),
        inner.to_frag(),
        anchor.r_angle_token().unwrap(),
    )
}

/// `<A, B>`, or the turbofish `::<A, B>`
fn format_generic_arg_list(fmt: &Formatter, args: GenericArgList) -> Frag {
    let mut builder = Builder::new(fmt);
    if let Some(token) = args.coloncolon_token() {
        builder.push_token(token, Tight);
    }
    let items = list_items_cast(args.syntax(), |a| format_generic_arg(fmt, a));
    let inner = flat_list(fmt, &items, None);
    builder.push(format_parenthesized(
        fmt,
        args.l_angle_token().unwrap(),
        inner,
        args.r_angle_token().unwrap(),
    ));
    builder.to_frag()
}

fn format_generic_arg(fmt: &Formatter, arg: GenericArg) -> Frag {
    match arg {
        GenericArg::TypeArg(arg) => format_type(fmt, arg.ty().unwrap()),
        GenericArg::LifetimeArg(arg) => fmt.token(arg.lifetime().unwrap().lifetime_ident_token().unwrap(), Word),
        GenericArg::ConstArg(arg) => format_expr(fmt, arg.expr().unwrap()),
        GenericArg::AssocTypeArg(arg) => format_assoc_type_arg(fmt, arg),
    }
}

/// `Item = u32` in `Iterator<Item = u32>`
fn format_assoc_type_arg(fmt: &Formatter, arg: AssocTypeArg) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_name_ref(fmt, arg.name_ref().unwrap()));
    if let Some(args) = arg.generic_arg_list() {
        builder.push(format_generic_arg_list(fmt, args));
    }
    let (Some(eq_token), None) = (arg.eq_token(), arg.type_bound_list()) else {
        panic!("{arg:?}");
    };
    builder.push_token(eq_token, Operator);
    match (arg.ty(), arg.const_arg()) {
        (Some(ty), _) => builder.push(format_type(fmt, ty)),
        (None, Some(arg)) => builder.push(format_expr(fmt, arg.expr().unwrap())),
        (None, None) => panic!("{arg:?}"),
    }
    builder.to_frag()
}

/// `(A, B)` in `Fn(A, B) -> C`
fn format_parenthesized_arg_list(fmt: &Formatter, args: ParenthesizedArgList) -> Frag {
    let mut builder = Builder::new(fmt);
    if let Some(token) = args.coloncolon_token() {
        builder.push_token(token, Tight);
    }
    let items = list_items_cast::<TypeArg, Frag>(args.syntax(), |a| {
        format_type(fmt, a.ty().unwrap())
    });
    let inner = flat_list(fmt, &items, None);
    builder.push(format_parenthesized(
        fmt,
        args.l_paren_token().unwrap(),
        inner,
        args.r_paren_token().unwrap(),
    ));
    builder.to_frag()
}
