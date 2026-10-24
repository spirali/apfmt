//! Types

use super::expr::format_expr;
use super::expr_list::single_item_separator;
use super::format_unsupported_kind;
use super::generics::{format_for_binder, format_type_bound_list};
use super::item::{format_param_list, push_qualifiers};
use super::path::format_path;
use crate::format::common::format_repeat;
use crate::formatter::Edge::{Operator, Tight, Word};
use crate::formatter::{Builder, Formatter, Frag};
use crate::layout::list::{Delimiters, FlatLimit, ListLayout, delimited_list, list_items_cast};
use ra_ap_syntax::AstNode;
use ra_ap_syntax::SyntaxToken;
use ra_ap_syntax::ast::{
    ArrayType, FnPtrType, ParenType, PtrType, RefType, RetType, SliceType, TupleType, Type,
    TypeBoundList,
};

/// `-> T`; broken only as a last resort, after the parameters in front of it (`fn f(` / `...` /
/// `) -> Result<A, B>`, `FnMut(` / `...` / `) -> ControlFlow<A, B>`, as the Style Guide breaks a
/// signature): a broken return type costs a second split
pub(super) fn format_ret_type(fmt: &Formatter, ret_type: RetType) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_token(ret_type.thin_arrow_token().unwrap(), Operator);
    builder.push(format_type(fmt, ret_type.ty().unwrap()));
    fmt.keep_together(builder.into_frag())
}

fn format_ref_type(fmt: &Formatter, ty: RefType) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_token(ty.amp_token().unwrap(), Tight);
    if let Some(lifetime) = ty.lifetime() {
        // always followed by a space, as a keyword: `&'a [T]`, `&'a (A, B)`
        builder.push_keyword(lifetime.lifetime_ident_token().unwrap());
    }
    if let Some(token) = ty.mut_token() {
        builder.push_keyword(token);
    }
    builder.push(format_type(fmt, ty.ty().unwrap()));
    builder.into_frag()
}

/// `*const T` or `*mut T`
fn format_ptr_type(fmt: &Formatter, ty: PtrType) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_token(ty.star_token().unwrap(), Tight);
    builder.push_keyword(ty.const_token().or(ty.mut_token()).unwrap());
    builder.push(format_type(fmt, ty.ty().unwrap()));
    builder.into_frag()
}

/// `(A, B)`, `(A,)` or `()`; a type, so kept on one line if possible
fn format_tuple_type(fmt: &Formatter, ty: TupleType) -> Frag {
    let fields = list_items_cast(ty.syntax(), |t| format_type(fmt, t));
    let layout = ListLayout::tall(FlatLimit::MaxWidth)
        .trailing_separator(single_item_separator(fields.len()));
    let delims = Delimiters::parens_of(ty.syntax());
    fmt.keep_together(delimited_list(fmt, &delims, &fields, layout))
}

/// `[T]`
fn format_slice_type(fmt: &Formatter, ty: SliceType) -> Frag {
    let inner = format_type(fmt, ty.ty().unwrap());
    Delimiters::brackets_of(ty.syntax()).enclose(fmt, inner)
}

/// `[T; N]`
fn format_array_type(fmt: &Formatter, ty: ArrayType) -> Frag {
    let value = format_type(fmt, ty.ty().unwrap());
    let len = format_expr(fmt, ty.const_arg().unwrap().expr().unwrap());
    format_repeat(fmt, ty.syntax(), value, ty.semicolon_token().unwrap(), len)
}

/// `(T)`, e.g. `&(dyn A + B)`
fn format_paren_type(fmt: &Formatter, ty: ParenType) -> Frag {
    let inner = format_type(fmt, ty.ty().unwrap());
    Delimiters::parens_of(ty.syntax()).enclose(fmt, inner)
}

/// `impl A + B`, `dyn A + B`, or a trait object without `dyn` (`A + B`, before edition 2018)
fn format_bounds_type(
    fmt: &Formatter,
    keyword: Option<SyntaxToken>,
    bounds: TypeBoundList,
) -> Frag {
    let mut builder = Builder::new(fmt);
    if let Some(keyword) = keyword {
        builder.push_keyword(keyword);
    }
    builder.push(format_type_bound_list(fmt, bounds));
    builder.into_frag()
}

/// `unsafe extern "C" fn(A, B) -> C`
fn format_fn_ptr_type(fmt: &Formatter, ty: FnPtrType) -> Frag {
    let mut builder = Builder::new(fmt);
    let fn_token = ty.fn_token().unwrap();
    push_qualifiers(fmt, &mut builder, ty.syntax(), &fn_token);
    builder.push_token(fn_token, Word);
    builder.push(format_param_list(fmt, ty.param_list().unwrap()));
    if let Some(ret_type) = ty.ret_type() {
        builder.push(format_ret_type(fmt, ret_type));
    }
    builder.into_frag()
}

pub(super) fn format_type(fmt: &Formatter, ty: Type) -> Frag {
    match ty {
        Type::PathType(ty) => format_path(fmt, ty.path().unwrap()),
        Type::RefType(ty) => format_ref_type(fmt, ty),
        Type::PtrType(ty) => format_ptr_type(fmt, ty),
        Type::TupleType(ty) => format_tuple_type(fmt, ty),
        Type::SliceType(ty) => format_slice_type(fmt, ty),
        Type::ArrayType(ty) => format_array_type(fmt, ty),
        Type::ParenType(ty) => format_paren_type(fmt, ty),
        Type::ImplTraitType(ty) => {
            format_bounds_type(fmt, ty.impl_token(), ty.type_bound_list().unwrap())
        }
        Type::DynTraitType(ty) => {
            format_bounds_type(fmt, ty.dyn_token(), ty.type_bound_list().unwrap())
        }
        Type::FnPtrType(ty) => format_fn_ptr_type(fmt, ty),
        Type::NeverType(ty) => fmt.token(ty.excl_token().unwrap(), Word),
        Type::InferType(ty) => fmt.token(ty.underscore_token().unwrap(), Word),
        Type::ForType(ty) => Builder::concat(
            fmt,
            [
                format_for_binder(fmt, ty.for_binder().unwrap()),
                format_type(fmt, ty.ty().unwrap()),
            ],
        ),
        ty => format_unsupported_kind(fmt, ty.syntax()),
    }
}
