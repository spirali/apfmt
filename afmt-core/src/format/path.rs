//! Paths and generic arguments

use super::expr::format_expr;
use super::format_unsupported;
use super::ty::{format_ret_type, format_type};
use crate::format::common::format_lifetime;
use crate::formatter::Edge::{Operator, Tight, Word};
use crate::formatter::{Builder, Formatter, Frag};
use crate::layout::list::{Delimiters, FlatLimit, ListLayout, delimited_list, list_items_cast};
use ra_ap_syntax::AstNode;
use ra_ap_syntax::ast::{
    AssocTypeArg, GenericArg, GenericArgList, HasGenericArgs, HasTypeBounds, NameRef,
    ParenthesizedArgList, Path, PathSegment, Type, TypeAnchor, TypeArg,
};

/// `a::b::c`; the path is left-recursive: `(a::b)::c`
pub(super) fn format_path(fmt: &Formatter, path: Path) -> Frag {
    let mut builder = Builder::new(fmt);
    if let Some(qualifier) = path.qualifier() {
        builder.push(format_path(fmt, qualifier));
        builder.push_token(path.coloncolon_token().unwrap(), Tight);
    }
    builder.push(format_path_segment(fmt, path.segment().unwrap()));
    builder.into_frag()
}

/// `name`, `name<args>`, `name::<args>`, `Fn(args) -> ret` or `<T as Trait>`; with a leading `::` in `::std`
fn format_path_segment(fmt: &Formatter, segment: PathSegment) -> Frag {
    if segment.return_type_syntax().is_some() {
        return format_unsupported(fmt, segment.syntax(), "return type notation");
    }
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
    builder.into_frag()
}

/// An identifier or one of `self`, `super`, `crate`, `Self`
pub(super) fn format_name_ref(fmt: &Formatter, name_ref: NameRef) -> Frag {
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
    Delimiters::angles_of(anchor.syntax()).enclose(fmt, inner.into_frag())
}

/// `<A, B>`, or the turbofish `::<A, B>`
pub(super) fn format_generic_arg_list(fmt: &Formatter, args: GenericArgList) -> Frag {
    let mut builder = Builder::new(fmt);
    if let Some(token) = args.coloncolon_token() {
        builder.push_token(token, Tight);
    }
    let items = list_items_cast(args.syntax(), |a| format_generic_arg(fmt, a));
    let delims = Delimiters::angles_of(args.syntax());
    let list = delimited_list(fmt, &delims, &items, ListLayout::tall(FlatLimit::MaxWidth));
    // "Avoid breaking lines in types where possible" (Style Guide)
    builder.push(fmt.keep_together(list));
    builder.into_frag()
}

fn format_generic_arg(fmt: &Formatter, arg: GenericArg) -> Frag {
    match arg {
        GenericArg::TypeArg(arg) => format_type(fmt, arg.ty().unwrap()),
        GenericArg::LifetimeArg(arg) => format_lifetime(fmt, arg.lifetime().unwrap()),
        GenericArg::ConstArg(arg) => format_expr(fmt, arg.expr().unwrap()),
        GenericArg::AssocTypeArg(arg) => format_assoc_type_arg(fmt, arg),
    }
}

/// `Item = u32` in `Iterator<Item = u32>`
fn format_assoc_type_arg(fmt: &Formatter, arg: AssocTypeArg) -> Frag {
    let (Some(eq_token), None) = (arg.eq_token(), arg.type_bound_list()) else {
        return format_unsupported(fmt, arg.syntax(), "an associated type bound");
    };
    let mut builder = Builder::new(fmt);
    builder.push(format_name_ref(fmt, arg.name_ref().unwrap()));
    if let Some(args) = arg.generic_arg_list() {
        builder.push(format_generic_arg_list(fmt, args));
    }
    builder.push_token(eq_token, Operator);
    match (arg.ty(), arg.const_arg()) {
        (Some(ty), _) => builder.push(format_type(fmt, ty)),
        (None, Some(arg)) => builder.push(format_expr(fmt, arg.expr().unwrap())),
        (None, None) => panic!("{arg:?}"),
    }
    builder.into_frag()
}

/// `(A, B)` in `Fn(A, B) -> C`
fn format_parenthesized_arg_list(fmt: &Formatter, args: ParenthesizedArgList) -> Frag {
    let mut builder = Builder::new(fmt);
    if let Some(token) = args.coloncolon_token() {
        builder.push_token(token, Tight);
    }
    let items =
        list_items_cast::<TypeArg, Frag>(args.syntax(), |a| format_type(fmt, a.ty().unwrap()));
    let delims = Delimiters::parens_of(args.syntax());
    let list = delimited_list(fmt, &delims, &items, ListLayout::tall(FlatLimit::MaxWidth));
    // "Avoid breaking lines in types where possible" (Style Guide)
    builder.push(fmt.keep_together(list));
    builder.into_frag()
}
