//! Paths and generic arguments

use super::expr::format_expr;
use super::format_unsupported;
use super::generics::format_angle_list;
use super::ty::{format_ret_type, format_type};
use crate::format::common::format_lifetime;
use crate::formatter::Edge::{Operator, Tight, Word};
use crate::formatter::{Builder, Formatter, Frag};
use crate::layout::list::{
    Delimiters, FlatLimit, Hug, ListLayout, delimited_list, hug_last_item, list_items_cast,
};
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

/// `name`, `name<args>`, `name::<args>`, `Fn(args) -> ret` or `<T as Trait>`, possibly after `::`
fn format_path_segment(fmt: &Formatter, segment: PathSegment) -> Frag {
    if segment.return_type_syntax().is_some() {
        return format_unsupported(fmt, segment.syntax(), "return type notation");
    }
    let mut builder = Builder::new(fmt);
    if let Some(token) = segment.coloncolon_token() {
        // Spaced as the name it stands in front of: `impl ::std::X`, `&::a`
        builder.push(fmt.token_with_edges(token, Word, Tight));
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

/// A single generic argument that may overflow its `<>`: a tuple type, also after `&` or `*`, as in
/// rustfmt (`From<&(` / `...` / `)>`)
fn is_combinable_type(ty: &Type) -> bool {
    match ty {
        Type::TupleType(_) => true,
        Type::RefType(ty) => ty.ty().is_some_and(|t| is_combinable_type(&t)),
        Type::PtrType(ty) => ty.ty().is_some_and(|t| is_combinable_type(&t)),
        _ => false,
    }
}

/// A single combinable type when that takes fewer lines; otherwise broken at the outermost scope
/// (Style Guide): `Option<` / `(A, B),` / `>`
fn last_type_arg_hug(args: &GenericArgList) -> Hug {
    let mut args = args.generic_args();
    match (args.next(), args.next()) {
        (Some(GenericArg::TypeArg(arg)), None)
            if arg.ty().is_some_and(|t| is_combinable_type(&t)) =>
        {
            Hug::Allowed
        }
        _ => Hug::Never,
    }
}

pub(super) fn format_generic_arg_list(fmt: &Formatter, args: GenericArgList) -> Frag {
    let mut builder = Builder::new(fmt);
    if let Some(token) = args.coloncolon_token() {
        builder.push_token(token, Tight);
    }
    let hug = last_type_arg_hug(&args);
    let mut items = list_items_cast(args.syntax(), |a| format_generic_arg(fmt, a));
    hug_last_item(&mut items, hug);
    builder.push(format_angle_list(fmt, args.syntax(), &items));
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

#[cfg(test)]
mod tests {
    use super::{Hug, last_type_arg_hug};
    use ra_ap_syntax::ast::GenericArgList;
    use ra_ap_syntax::{AstNode, Edition, SourceFile};

    fn hug(ty: &str) -> Hug {
        let source = format!("type T = {ty};");
        let parse = SourceFile::parse(&source, Edition::Edition2024);
        let list = (parse.syntax_node().descendants())
            .find_map(GenericArgList::cast)
            .unwrap();
        last_type_arg_hug(&list)
    }

    #[test]
    fn last_type_arg_hugs() {
        use Hug::{Allowed, Never};
        for (ty, expected) in [
            ("Option<(A, B)>", Allowed),
            ("From<&(A, B)>", Allowed),
            ("From<&mut (A, B)>", Allowed),
            ("Box<*const (A, B)>", Allowed),
            ("Vec<A>", Never),
            ("Vec<[A; 2]>", Never),
            ("Vec<&[A]>", Never),
            ("Vec<Foo<A, B>>", Never),
            ("HashMap<K, (A, B)>", Never),
            ("Foo<'a, (A, B)>", Never),
        ] {
            assert_eq!(hug(ty), expected, "{ty}");
        }
    }
}
