//! Items: functions, structs, enums

use super::expr::format_expr;
use super::generics::{format_generic_param_list, format_where_clause};
use super::pat::format_pat;
use super::path::format_path;
use super::stmt::format_block_expr;
use super::ty::{format_ret_type, format_type};
use crate::format::common::{format_lifetime, format_name};
use crate::formatter::Edge::{Comma, Operator, Tight, Word};
use crate::formatter::{Builder, Formatter, Frag};
use crate::layout::list::{
    Delimiters, FlatLimit, ListItem, ListLayout, TrailingSeparator, delimited_list, flat_layout,
    list_items, list_items_cast,
};
use ra_ap_syntax::ast::{
    Abi, Enum, FieldList, Fn as AstFn, HasGenericParams, HasName, HasVisibility, Item, Param,
    ParamList, RecordField, RecordFieldList, SelfParam, Struct, TupleField, TupleFieldList,
    Variant, Visibility, WhereClause,
};
use ra_ap_syntax::{AstNode, NodeOrToken, SyntaxKind, SyntaxToken};

/// `self`, `&self`, `&'a mut self`, `mut self` or `self: Type`
fn format_self_param(fmt: &Formatter, param: SelfParam) -> Frag {
    let mut builder = Builder::new(fmt);
    if let Some(token) = param.amp_token() {
        builder.push_token(token, Tight);
    }
    if let Some(lifetime) = param.lifetime() {
        builder.push(format_lifetime(fmt, lifetime));
    }
    if let Some(token) = param.mut_token() {
        builder.push_token(token, Word);
    }
    builder.push_token(param.name().unwrap().self_token().unwrap(), Word);
    if let Some(ty) = param.ty() {
        builder.push_token(param.colon_token().unwrap(), Comma);
        builder.push(format_type(fmt, ty));
    }
    builder.into_frag()
}

/// `pat: Type`
fn format_param(fmt: &Formatter, param: Param) -> Frag {
    let (Some(pat), Some(ty), None) = (param.pat(), param.ty(), param.dotdotdot_token()) else {
        panic!("{param:?}");
    };
    let mut builder = Builder::new(fmt);
    builder.push(format_pat(fmt, pat));
    builder.push_token(param.colon_token().unwrap(), Comma);
    builder.push(format_type(fmt, ty));
    builder.into_frag()
}

/// `(a: A, b: B)` on a single line, or one parameter per line with a trailing comma
fn format_param_list(fmt: &Formatter, param_list: ParamList) -> Frag {
    let params = list_items(param_list.syntax(), |n| match n.kind() {
        SyntaxKind::SELF_PARAM => Some(format_self_param(fmt, SelfParam::cast(n).unwrap())),
        SyntaxKind::PARAM => Some(format_param(fmt, Param::cast(n).unwrap())),
        _ => None,
    });
    let delims = Delimiters::parens_of(param_list.syntax());
    delimited_list(fmt, &delims, &params, ListLayout::tall(FlatLimit::MaxWidth))
}

/// The head of an item up to its generic parameters: the visibility, the qualifiers (`default const
/// async gen unsafe safe extern "abi"`, in the source order), the keyword, the name and the generic
/// parameters, e.g. `pub const unsafe fn name<T>`, `pub struct Name<T>`
fn format_item_head<N: HasName + HasGenericParams>(
    fmt: &Formatter,
    item: &N,
    keyword: SyntaxToken,
) -> Frag {
    let mut builder = Builder::new(fmt);
    for child in item.syntax().children_with_tokens() {
        match child {
            NodeOrToken::Token(token) if token == keyword => break,
            NodeOrToken::Token(token) => match token.kind() {
                kind if kind.is_trivia() => {}
                SyntaxKind::DEFAULT_KW
                | SyntaxKind::CONST_KW
                | SyntaxKind::ASYNC_KW
                | SyntaxKind::GEN_KW
                | SyntaxKind::UNSAFE_KW
                | SyntaxKind::SAFE_KW => builder.push_token(token, Word),
                _ => panic!("not supported yet: {token:?}"),
            },
            NodeOrToken::Node(node) => match node.kind() {
                SyntaxKind::VISIBILITY => {
                    builder.push(format_visibility(fmt, Visibility::cast(node).unwrap()))
                }
                SyntaxKind::ABI => builder.push(format_abi(fmt, Abi::cast(node).unwrap())),
                // Attributes, ...: they would be lost
                _ => panic!("not supported yet: {node:?}"),
            },
        }
    }
    builder.push_token(keyword, Word);
    builder.push(format_name(fmt, item.name().unwrap()));
    if let Some(generics) = item.generic_param_list() {
        builder.push(format_generic_param_list(fmt, generics));
    }
    builder.into_frag()
}

/// `extern "abi"`; a missing ABI is written out, `extern` is `extern "C"` (Style Guide: "always specify
/// the ABI")
fn format_abi(fmt: &Formatter, abi: Abi) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_token(abi.extern_token().unwrap(), Word);
    match abi.string_token() {
        Some(token) => builder.push_token(token, Word),
        None => builder.push(fmt.text("\"C\"", Word)),
    }
    builder.into_frag()
}

/// The `where` clause of an item on the next line; with `terminated`, the item ends with `;` right
/// after it, otherwise its body follows on the next line
fn push_where_clause(
    fmt: &Formatter,
    builder: &mut Builder,
    clause: Option<WhereClause>,
    terminated: bool,
) {
    if let Some(clause) = clause {
        builder.push(fmt.on_next_line(format_where_clause(fmt, clause, terminated)));
    }
}

/// `fn name<T>(params) -> Ret where ... { body }`, or with `;` instead of the body
fn format_fn(fmt: &Formatter, item_fn: AstFn) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_item_head(fmt, &item_fn, item_fn.fn_token().unwrap()));
    builder.push(format_param_list(fmt, item_fn.param_list().unwrap()));
    if let Some(ret_type) = item_fn.ret_type() {
        builder.push(format_ret_type(fmt, ret_type));
    }
    let body = item_fn.body();
    push_where_clause(fmt, &mut builder, item_fn.where_clause(), body.is_none());
    match body {
        Some(body) => builder.push(format_block_expr(fmt, body)),
        None => builder.push_separator(SyntaxKind::SEMICOLON),
    }
    builder.into_frag()
}

/// `pub`, `pub(crate)`, `pub(super)` or `pub(in path)`
fn format_visibility(fmt: &Formatter, visibility: Visibility) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_token(visibility.pub_token().unwrap(), Word);
    if let Some(inner) = visibility.visibility_inner() {
        let mut path = Builder::new(fmt);
        if let Some(token) = inner.in_token() {
            path.push_keyword(token);
        }
        path.push(format_path(fmt, inner.path().unwrap()));
        builder.push(Delimiters::parens_of(inner.syntax()).enclose(fmt, path.into_frag()));
    }
    builder.into_frag()
}

/// `pub name: Type`
fn format_record_field(fmt: &Formatter, field: RecordField) -> Frag {
    let mut builder = Builder::new(fmt);
    if let Some(visibility) = field.visibility() {
        builder.push(format_visibility(fmt, visibility));
    }
    builder.push(format_name(fmt, field.name().unwrap()));
    builder.push_token(field.colon_token().unwrap(), Comma);
    builder.push(format_type(fmt, field.ty().unwrap()));
    builder.into_frag()
}

/// `pub Type`
fn format_tuple_field(fmt: &Formatter, field: TupleField) -> Frag {
    let mut builder = Builder::new(fmt);
    if let Some(visibility) = field.visibility() {
        builder.push(format_visibility(fmt, visibility));
    }
    builder.push(format_type(fmt, field.ty().unwrap()));
    builder.into_frag()
}

/// `{ a: A, b: B }` of a struct: always one field per line
fn format_record_field_list(fmt: &Formatter, list: RecordFieldList) -> Frag {
    let fields = list_items_cast(list.syntax(), |f| format_record_field(fmt, f));
    let delims = Delimiters::braces_of(list.syntax());
    delimited_list(fmt, &delims, &fields, ListLayout::fixed_vertical())
}

/// `(A, B)` of a tuple struct or a tuple variant: on a single line if the fields fit within
/// `fn_call_width`, otherwise one field per line with a trailing comma
fn format_tuple_field_list(fmt: &Formatter, list: TupleFieldList) -> Frag {
    let fields = list_items_cast(list.syntax(), |f| format_tuple_field(fmt, f));
    let delims = Delimiters::parens_of(list.syntax());
    delimited_list(
        fmt,
        &delims,
        &fields,
        ListLayout::tall(FlatLimit::Within(fmt.config().fn_call_width)),
    )
}

/// `struct Name;`, `struct Name { a: A }` or `struct Name(A);`
fn format_struct(fmt: &Formatter, item: Struct) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_item_head(fmt, &item, item.struct_token().unwrap()));
    // The `where` clause is in front of the fields in `struct S<T> where ... { a: T }`, and after
    // them, followed by `;`, in `struct S<T>(T) where ...;`
    match item.field_list() {
        Some(FieldList::RecordFieldList(list)) => {
            push_where_clause(fmt, &mut builder, item.where_clause(), false);
            builder.push(format_record_field_list(fmt, list));
        }
        Some(FieldList::TupleFieldList(list)) => {
            builder.push(format_tuple_field_list(fmt, list));
            push_where_clause(fmt, &mut builder, item.where_clause(), true);
        }
        None => push_where_clause(fmt, &mut builder, item.where_clause(), true),
    }
    if item.semicolon_token().is_some() {
        builder.push_separator(SyntaxKind::SEMICOLON);
    }
    builder.into_frag()
}

/// `{ a: A, b: B }` of a struct variant, which is put on a single line or one field per line together
/// with the other struct variants of the enum
struct StructVariantFields {
    delims: Delimiters,
    fields: Vec<ListItem>,
    /// The fields on a single line, if they fit within `struct_variant_width`
    single_line: Option<Frag>,
}

enum VariantFields {
    Unit,
    Tuple(Frag),
    Struct(StructVariantFields),
}

/// A variant formatted up to the layout of its struct fields
struct VariantParts {
    name: Frag,
    fields: VariantFields,
    /// `= expr`
    discriminant: Option<Frag>,
}

fn struct_variant_fields(fmt: &Formatter, list: RecordFieldList) -> StructVariantFields {
    let delims = Delimiters::braces_of(list.syntax());
    let fields = list_items_cast(list.syntax(), |f| format_record_field(fmt, f));
    let single_line = if fields.is_empty() {
        // `{}`, unless it contains comments
        let has_comment = list
            .syntax()
            .children_with_tokens()
            .any(|e| e.kind() == SyntaxKind::COMMENT);
        (!has_comment).then(Frag::empty)
    } else {
        let width = fmt.config().struct_variant_width;
        let limit = FlatLimit::Within(width);
        flat_layout(fmt, &delims, &fields, limit, TrailingSeparator::IfVertical)
    };
    StructVariantFields {
        delims,
        fields,
        single_line,
    }
}

fn variant_parts(fmt: &Formatter, variant: Variant) -> VariantParts {
    let fields = match variant.field_list() {
        Some(FieldList::RecordFieldList(list)) => {
            VariantFields::Struct(struct_variant_fields(fmt, list))
        }
        Some(FieldList::TupleFieldList(list)) => {
            VariantFields::Tuple(format_tuple_field_list(fmt, list))
        }
        None => VariantFields::Unit,
    };
    let discriminant = variant.eq_token().map(|eq_token| {
        let mut builder = Builder::new(fmt);
        builder.push_token(eq_token, Operator);
        builder.push(format_expr(
            fmt,
            variant.const_arg().unwrap().expr().unwrap(),
        ));
        builder.into_frag()
    });
    VariantParts {
        name: format_name(fmt, variant.name().unwrap()),
        fields,
        discriminant,
    }
}

/// `Name`, `Name(A, B)`, `Name { a: A }` or `Name = discriminant`; with `single_line_structs`, the struct
/// fields are on a single line
fn format_variant(fmt: &Formatter, parts: &VariantParts, single_line_structs: bool) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(parts.name.clone());
    match &parts.fields {
        VariantFields::Unit => {}
        VariantFields::Tuple(fields) => builder.push(fields.clone()),
        VariantFields::Struct(s) if single_line_structs => {
            builder.push(s.delims.wrap_body(fmt, s.single_line.clone().unwrap()));
        }
        VariantFields::Struct(s) => builder.push(delimited_list(
            fmt,
            &s.delims,
            &s.fields,
            ListLayout::fixed_vertical(),
        )),
    }
    if let Some(discriminant) = &parts.discriminant {
        builder.push(discriminant.clone());
    }
    builder.into_frag()
}

fn format_enum(fmt: &Formatter, item: Enum) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_item_head(fmt, &item, item.enum_token().unwrap()));
    push_where_clause(fmt, &mut builder, item.where_clause(), false);

    let list = item.variant_list().unwrap();
    let delims = Delimiters::braces_of(list.syntax());
    let variants = list_items_cast(list.syntax(), |v| variant_parts(fmt, v));
    let body = |single_line_structs: bool| {
        let items = variants
            .iter()
            .map(|v| v.map(|parts| format_variant(fmt, parts, single_line_structs)))
            .collect::<Vec<ListItem>>();
        delimited_list(fmt, &delims, &items, ListLayout::fixed_vertical())
    };
    let struct_fields = || {
        variants.iter().filter_map(|v| match &v.frag.fields {
            VariantFields::Struct(s) => Some(s),
            _ => None,
        })
    };
    let all_small =
        struct_fields().next().is_some() && struct_fields().all(|s| s.single_line.is_some());
    // The vertical struct variants stay as a fallback when a single-line one exceeds `max_width`
    let vertical = body(false);
    builder.push(if all_small {
        fmt.alt_frag(body(true), vertical)
    } else {
        vertical
    });
    builder.into_frag()
}

pub(super) fn format_item(fmt: &Formatter, item: Item) -> Frag {
    let mut builder = Builder::new(fmt);
    match item {
        Item::Fn(item) => builder.push(format_fn(fmt, item)),
        Item::Struct(item) => builder.push(format_struct(fmt, item)),
        Item::Enum(item) => builder.push(format_enum(fmt, item)),
        x => {
            panic!("{x:?}");
        }
    }
    builder.into_frag()
}
