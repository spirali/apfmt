//! Items: functions, structs, enums, use declarations, impls, traits, modules, constants, type aliases

use super::attr::{attach_attrs, format_attr, format_skippable_with_attrs, format_with_attrs};
use super::expr::format_expr;
use super::generics::{
    format_generic_param_list, format_type_bound_list, format_unindented_type_bound_list,
    format_where_clause,
};
use super::macros::{format_macro_call, format_macro_rules};
use super::pat::format_pat;
use super::path::format_path;
use super::stmt::format_block_expr;
use super::ty::{format_ret_type, format_type};
use super::{format_unsupported, format_unsupported_kind};
use crate::comments::{is_code_kind, is_comment_kind};
use crate::format::common::{format_lifetime, format_name};
use crate::formatter::Edge::{Comma, Operator, Tight, Word};
use crate::formatter::{Builder, Formatter, Frag};
use crate::layout::list::{
    Delimiters, FlatLimit, ListItem, ListLayout, Overflow, TrailingSeparator, delimited_list,
    flat_layout, list_items, list_items_cast,
};
use crate::syntax_utils::has_attrs;
use ra_ap_syntax::ast::{
    Abi, AssocItemList, Const, Enum, Expr, ExternBlock, FieldList, Fn as AstFn, GenericParamList,
    HasAttrs, HasGenericParams, HasModuleItem, HasName, HasTypeBounds, HasVisibility, Impl, Item,
    ItemList, Module, Param, ParamList, Pat, RecordField, RecordFieldList, Rename, SelfParam,
    Static, Struct, Trait, TupleField, TupleFieldList, Type, TypeAlias, TypeBoundList, Use,
    UseTree, UseTreeList, Variant, Visibility, WhereClause,
};
use ra_ap_syntax::{AstNode, NodeOrToken, SyntaxKind, SyntaxNode, SyntaxToken};

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

/// `pat: Type`, just `Type` in a function pointer type, or just `pat` in a closure
fn format_param(fmt: &Formatter, param: Param) -> Frag {
    if param.dotdotdot_token().is_some() {
        return format_unsupported(fmt, param.syntax(), "a variadic parameter");
    }
    let mut builder = Builder::new(fmt);
    if let Some(pat) = param.pat() {
        builder.push(format_pat(fmt, pat));
    }
    if let Some(ty) = param.ty() {
        if let Some(colon) = param.colon_token() {
            builder.push_token(colon, Comma);
        }
        builder.push(format_type(fmt, ty));
    }
    builder.into_frag()
}

/// `(a: A, b: B)` on a single line, or one parameter per line with a trailing comma
pub(super) fn format_param_list(fmt: &Formatter, param_list: ParamList) -> Frag {
    let params = param_items(fmt, &param_list);
    let delims = Delimiters::parens_of(param_list.syntax());
    delimited_list(fmt, &delims, &params, ListLayout::tall(FlatLimit::MaxWidth))
}

/// `|a, b: B|` of a closure on a single line (`None` if a comment prevents it), and broken (`None`
/// without parameters, or with a single one that fits on a single line): aligned after `|` as rustfmt
/// does (`|a: A,` / ` b: B|`), an explicit exception to the Style Guide's "function definition syntax"
/// (see CLAUDE.md). A single tuple-like parameter is hugged instead (see `is_hugged_closure_param`).
pub(super) fn closure_params_layouts(
    fmt: &Formatter,
    param_list: ParamList,
) -> (Option<Frag>, Option<Frag>) {
    let mut param_nodes = param_list.params();
    let single = param_nodes.next().filter(|_| param_nodes.next().is_none());
    let hugged = single.as_ref().is_some_and(is_hugged_closure_param);
    let mut params = param_items(fmt, &param_list);
    let delims = Delimiters::pipes_of(param_list.syntax());
    if params.is_empty() {
        return (Some(delims.format_empty(fmt)), None);
    }
    if hugged {
        params[0].overflow = Overflow::Only(params[0].frag.clone());
    }
    let flat = flat_layout(
        fmt,
        &delims,
        &params,
        FlatLimit::MaxWidth,
        TrailingSeparator::IfVertical,
    )
    .map(|flat| delims.wrap_body(fmt, flat));
    // A single parameter on one line is the same broken, unless it is hugged; one that cannot be on
    // one line (`|S {` / `    long_field,` / `}|`, or a comment forcing a line break) has only
    // broken layouts
    if single.is_some() && !hugged && flat.is_some() {
        return (flat, None);
    }
    let layout = ListLayout::tall(FlatLimit::MaxWidth)
        .broken_if(true)
        .aligned();
    (flat, Some(delimited_list(fmt, &delims, &params, layout)))
}

/// Whether the single parameter of a closure is hugged by the pipes when it does not fit on one
/// line, its elements block indented, as rustfmt does: a tuple, slice or tuple struct pattern, also
/// after `&` or followed by a type (`|(` / `    a,` / `    b,` / `)| body`); a struct pattern is
/// aligned after `|` like other parameters (`|S {` / `     a,` / ` }|`)
fn is_hugged_closure_param(param: &Param) -> bool {
    let mut pat = param.pat();
    while let Some(Pat::RefPat(ref_pat)) = &pat {
        pat = ref_pat.pat();
    }
    !has_attrs(param)
        && matches!(
            pat,
            Some(Pat::TuplePat(_) | Pat::SlicePat(_) | Pat::TupleStructPat(_))
        )
}

fn param_items(fmt: &Formatter, param_list: &ParamList) -> Vec<ListItem> {
    list_items(param_list.syntax(), |n| match n.kind() {
        SyntaxKind::SELF_PARAM => {
            let param = SelfParam::cast(n).unwrap();
            Some(format_with_attrs(fmt, &param, true, || {
                format_self_param(fmt, param.clone())
            }))
        }
        SyntaxKind::PARAM => {
            let param = Param::cast(n).unwrap();
            Some(format_with_attrs(fmt, &param, true, || {
                format_param(fmt, param.clone())
            }))
        }
        _ => None,
    })
}

/// The visibility and the qualifiers (`default const async gen unsafe safe extern "abi"`) of `node` in
/// the source order, up to `keyword` (exclusive); attributes are formatted by the caller
pub(super) fn push_qualifiers(
    fmt: &Formatter,
    builder: &mut Builder,
    node: &SyntaxNode,
    keyword: &SyntaxToken,
) {
    for child in node.children_with_tokens() {
        match child {
            NodeOrToken::Token(token) if &token == keyword => break,
            NodeOrToken::Token(token) => match token.kind() {
                kind if !is_code_kind(kind) => {}
                SyntaxKind::DEFAULT_KW
                | SyntaxKind::CONST_KW
                | SyntaxKind::ASYNC_KW
                | SyntaxKind::GEN_KW
                | SyntaxKind::UNSAFE_KW
                | SyntaxKind::SAFE_KW
                | SyntaxKind::AUTO_KW => builder.push_token(token, Word),
                _ => panic!("not supported yet: {token:?}"),
            },
            NodeOrToken::Node(node) => match node.kind() {
                // Placed with the attributes and comments in front of the item
                SyntaxKind::ATTR | SyntaxKind::DOC_COMMENT => {}
                SyntaxKind::VISIBILITY => {
                    builder.push(format_visibility(fmt, Visibility::cast(node).unwrap()))
                }
                SyntaxKind::ABI => builder.push(format_abi(fmt, Abi::cast(node).unwrap())),
                _ => panic!("not supported yet: {node:?}"),
            },
        }
    }
}

/// The head of an item up to its generic parameters: the visibility, the qualifiers (see
/// `push_qualifiers`), the keyword, the name and the generic parameters, e.g. `pub const unsafe fn
/// name<T>`, `pub struct Name<T>`
fn format_item_head(
    fmt: &Formatter,
    item: &SyntaxNode,
    keyword: SyntaxToken,
    name: Frag,
    generics: Option<GenericParamList>,
) -> Frag {
    let mut builder = Builder::new(fmt);
    push_qualifiers(fmt, &mut builder, item, &keyword);
    builder.push_token(keyword, Word);
    builder.push(name);
    if let Some(generics) = generics {
        builder.push(format_generic_param_list(fmt, generics));
    }
    builder.into_frag()
}

/// `format_item_head` of a named item
fn format_named_item_head<N: HasName + HasGenericParams>(
    fmt: &Formatter,
    item: &N,
    keyword: SyntaxToken,
) -> Frag {
    let name = format_name(fmt, item.name().unwrap());
    format_item_head(fmt, item.syntax(), keyword, name, item.generic_param_list())
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
    let head = format_named_item_head(fmt, &item_fn, item_fn.fn_token().unwrap());
    let param_list = item_fn.param_list().unwrap();
    let has_params = param_list.self_param().is_some() || param_list.params().next().is_some();
    let params = format_param_list(fmt, param_list);
    let signature = match item_fn.ret_type().map(|ret| format_ret_type(fmt, ret)) {
        // A return type that does not fit on one line breaks only after the parameters (Style
        // Guide, as rustfmt does): `fn f(` / `    a: A,` / `) -> Result<` / `...`
        Some(ret) if has_params => fmt.concat_breaking_in_order(params, ret),
        // Without parameters to break, the return type that does not fit moves to the next line
        // at the indentation of `fn`, as rustfmt does (`fn name()` / `-> Type {`), rather than
        // breaking its generic arguments or the generics
        Some(ret) => {
            let ret = fmt.alt_frag(ret.clone(), fmt.on_next_line(ret));
            Builder::concat(fmt, [params, ret])
        }
        None => params,
    };
    if item_fn.generic_param_list().is_some() && has_params {
        // broken generics are followed by broken parameters, as rustfmt does (the Style Guide does
        // not say): `>(` / `    a: A,` / `) {`
        builder.push(fmt.concat_breaking_together(head, signature));
    } else {
        builder.push(head);
        builder.push(signature);
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

/// `pub name: Type`, with its attributes on their own lines; the type moves to the next line if it does
/// not fit on the line of the name (RhsBreak)
fn format_record_field(fmt: &Formatter, field: RecordField) -> Frag {
    format_with_attrs(fmt, &field, false, || format_record_field_body(fmt, &field))
}

fn format_record_field_body(fmt: &Formatter, field: &RecordField) -> Frag {
    let mut builder = Builder::new(fmt);
    if let Some(visibility) = field.visibility() {
        builder.push(format_visibility(fmt, visibility));
    }
    builder.push(format_name(fmt, field.name().unwrap()));
    builder.push_token(field.colon_token().unwrap(), Comma);
    builder.push(fmt.same_or_indented_line(format_type(fmt, field.ty().unwrap())));
    builder.into_frag()
}

/// `pub Type`, with its attributes on the same line
fn format_tuple_field(fmt: &Formatter, field: TupleField) -> Frag {
    format_with_attrs(fmt, &field, true, || {
        let mut builder = Builder::new(fmt);
        if let Some(visibility) = field.visibility() {
            builder.push(format_visibility(fmt, visibility));
        }
        builder.push(format_type(fmt, field.ty().unwrap()));
        builder.into_frag()
    })
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
    builder.push(format_named_item_head(
        fmt,
        &item,
        item.struct_token().unwrap(),
    ));
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
    list: RecordFieldList,
    delims: Delimiters,
    fields: Vec<ListItem>,
}

enum VariantFields {
    Unit,
    Tuple(Frag),
    Struct(StructVariantFields),
}

/// A variant formatted up to the layout of its struct fields
struct VariantParts {
    /// The attributes, each on its own line
    attrs: Vec<Frag>,
    name: Frag,
    fields: VariantFields,
    /// `= expr`
    discriminant: Option<Frag>,
}

fn struct_variant_fields(fmt: &Formatter, list: RecordFieldList) -> StructVariantFields {
    let delims = Delimiters::braces_of(list.syntax());
    let fields = list_items_cast(list.syntax(), |f| format_record_field(fmt, f));
    StructVariantFields {
        list,
        delims,
        fields,
    }
}

/// The fields of a struct variant on a single line, if they fit within `struct_variant_width`
fn struct_variant_single_line(fmt: &Formatter, s: &StructVariantFields) -> Option<Frag> {
    if s.fields.is_empty() {
        // `{}`, unless it contains comments
        let has_comment = s
            .list
            .syntax()
            .children_with_tokens()
            .any(|e| is_comment_kind(e.kind()));
        return (!has_comment).then(Frag::empty);
    }
    let limit = FlatLimit::Within(fmt.config().struct_variant_width);
    flat_layout(
        fmt,
        &s.delims,
        &s.fields,
        limit,
        TrailingSeparator::IfVertical,
    )
}

fn variant_parts(fmt: &Formatter, variant: Variant) -> VariantParts {
    let attrs = variant.attrs().map(|attr| format_attr(fmt, attr)).collect();
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
        attrs,
        name: format_name(fmt, variant.name().unwrap()),
        fields,
        discriminant,
    }
}

/// `Name`, `Name(A, B)`, `Name { a: A }` or `Name = discriminant`, after its attributes; with
/// `single_line`, the fields of a struct variant on a single line (see `struct_variant_single_line`),
/// otherwise one per line
fn format_variant(fmt: &Formatter, parts: &VariantParts, single_line: Option<&Frag>) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(parts.name.clone());
    match &parts.fields {
        VariantFields::Unit => {}
        VariantFields::Tuple(fields) => builder.push(fields.clone()),
        VariantFields::Struct(s) if single_line.is_some() => {
            builder.push(s.delims.wrap_body(fmt, single_line.unwrap().clone()));
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
    attach_attrs(fmt, parts.attrs.iter().cloned(), false, builder.into_frag())
}

fn format_enum(fmt: &Formatter, item: Enum) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_named_item_head(
        fmt,
        &item,
        item.enum_token().unwrap(),
    ));
    push_where_clause(fmt, &mut builder, item.where_clause(), false);

    let list = item.variant_list().unwrap();
    let delims = Delimiters::braces_of(list.syntax());
    let variants = list_items_cast(list.syntax(), |v| variant_parts(fmt, v));
    // The other variants than struct variants are the same in both bodies, so they are formatted once
    let fixed = variants
        .iter()
        .map(|v| match v.frag.fields {
            VariantFields::Struct(_) => None,
            _ => Some(format_variant(fmt, &v.frag, None)),
        })
        .collect::<Vec<_>>();
    // `single_lines`: the single-line fields of the struct variants, in their order
    let body = |single_lines: Option<&[Frag]>| {
        let mut single_lines = single_lines.map(|lines| lines.iter());
        let items = variants
            .iter()
            .zip(&fixed)
            .map(|(v, fixed)| {
                v.map(|parts| match fixed {
                    Some(fixed) => fixed.clone(),
                    None => {
                        let single_line = single_lines.as_mut().and_then(Iterator::next);
                        format_variant(fmt, parts, single_line)
                    }
                })
            })
            .collect::<Vec<ListItem>>();
        delimited_list(fmt, &delims, &items, ListLayout::fixed_vertical())
    };
    // All struct variants on a single line, or none (Style Guide); they are built one by one up to
    // the first that does not fit, so no fragment is built in vain (arena hygiene)
    let single_lines: Option<Vec<Frag>> = variants
        .iter()
        .filter_map(|v| match &v.frag.fields {
            VariantFields::Struct(s) => Some(s),
            _ => None,
        })
        .map(|s| struct_variant_single_line(fmt, s))
        .collect();
    let single_lines = single_lines.filter(|lines| !lines.is_empty());
    // The vertical struct variants stay as a fallback when a single-line one exceeds `max_width`
    let vertical = body(None);
    builder.push(match single_lines {
        Some(lines) => fmt.alt_frag(body(Some(&lines)), vertical),
        None => vertical,
    });
    builder.into_frag()
}

/// `use tree;`, `pub(crate) use tree;`
fn format_use(fmt: &Formatter, item: Use) -> Frag {
    let mut builder = Builder::new(fmt);
    if let Some(visibility) = item.visibility() {
        builder.push(format_visibility(fmt, visibility));
    }
    builder.push_keyword(item.use_token().unwrap());
    builder.push(format_use_tree(fmt, item.use_tree().unwrap()));
    builder.push_separator(SyntaxKind::SEMICOLON);
    builder.into_frag()
}

/// `a::b`, `a::b as c`, `a::*`, `a::{b, c}`, `::{a, b}` or `{a, b}`
fn format_use_tree(fmt: &Formatter, tree: UseTree) -> Frag {
    let mut builder = Builder::new(fmt);
    if let Some(path) = tree.path() {
        builder.push(format_path(fmt, path));
    }
    if let Some(token) = tree.coloncolon_token() {
        builder.push_token(token, Tight);
    }
    if let Some(token) = tree.star_token() {
        builder.push_token(token, Word);
    }
    if let Some(list) = tree.use_tree_list() {
        builder.push(format_use_tree_list(fmt, list));
    }
    if let Some(rename) = tree.rename() {
        builder.push(format_rename(fmt, rename));
    }
    builder.into_frag()
}

/// `as name` or `as _`
fn format_rename(fmt: &Formatter, rename: Rename) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_token(rename.as_token().unwrap(), Word);
    match (rename.name(), rename.underscore_token()) {
        (Some(name), _) => builder.push(format_name(fmt, name)),
        (None, Some(token)) => builder.push_token(token, Word),
        (None, None) => panic!("{rename:?}"),
    }
    builder.into_frag()
}

/// `{a, b}` of a use tree: on a single line if it fits, otherwise the names filled on lines (Style Guide:
/// "non-nested imports must be grouped on as few lines as possible"). If an item is a nested import
/// (`b::{c, d}`), the list is always broken and every item containing `::` is alone on its line.
fn format_use_tree_list(fmt: &Formatter, list: UseTreeList) -> Frag {
    let trees = list_items_cast(list.syntax(), |t: UseTree| t);
    let has_nested_list = trees.iter().any(|t| t.frag.use_tree_list().is_some());
    let items = trees
        .iter()
        .map(|tree| {
            // Paths (`a::b`, `a::*`) are alone on their line only next to nested imports, as in rustfmt
            let own_line = has_nested_list
                && tree
                    .frag
                    .syntax()
                    .descendants_with_tokens()
                    .any(|e| e.kind() == SyntaxKind::COLON2);
            tree.map(|t| format_use_tree(fmt, t.clone()))
                .own_line_if(own_line)
        })
        .collect::<Vec<ListItem>>();
    let delims = Delimiters::tight_braces_of(list.syntax());
    let layout = ListLayout::tall(FlatLimit::MaxWidth)
        .fill_if(true)
        .broken_if(has_nested_list);
    delimited_list(fmt, &delims, &items, layout)
}

/// `: A + B` of a trait: on the line of the head, or broken after `:` with the bounds on an indented
/// line, broken before every `+` at its indentation; then `{` of `block_follows` is on its own line
/// (Style Guide)
fn format_supertraits(
    fmt: &Formatter,
    colon: SyntaxToken,
    bounds: TypeBoundList,
    block_follows: bool,
) -> Frag {
    let bounds = format_unindented_type_bound_list(fmt, bounds);
    let mut broken = Builder::new(fmt);
    broken.push(fmt.indented_line(bounds.clone()));
    if block_follows {
        broken.push_newline();
    }
    let mut builder = Builder::new(fmt);
    builder.push_token(colon, Comma);
    builder.push(fmt.alt_frag(fmt.flat(bounds), broken.into_frag()));
    builder.into_frag()
}

/// `{ items }` of a module, an impl, a trait or an extern block: the inner attributes of `list`
/// and the `items`, each on its own line
fn format_item_block(
    fmt: &Formatter,
    list: &impl HasAttrs,
    items: impl Iterator<Item = SyntaxNode>,
) -> Frag {
    let lines = list
        .attrs()
        .map(|attr| format_attr(fmt, attr))
        .chain(items.map(|item| format_item(fmt, Item::cast(item).unwrap())));
    Delimiters::braces_of(list.syntax()).format_block(fmt, lines)
}

pub(super) fn format_item_list(fmt: &Formatter, list: ItemList) -> Frag {
    format_item_block(fmt, &list, list.items().map(|i| i.syntax().clone()))
}

fn format_assoc_item_list(fmt: &Formatter, list: AssocItemList) -> Frag {
    format_item_block(fmt, &list, list.assoc_items().map(|i| i.syntax().clone()))
}

/// `unsafe extern "C" { items }`
fn format_extern_block(fmt: &Formatter, item: ExternBlock) -> Frag {
    let mut builder = Builder::new(fmt);
    if let Some(token) = item.unsafe_token() {
        builder.push_token(token, Word);
    }
    builder.push(format_abi(fmt, item.abi().unwrap()));
    let list = item.extern_item_list().unwrap();
    builder.push(format_item_block(
        fmt,
        &list,
        list.extern_items().map(|i| i.syntax().clone()),
    ));
    builder.into_frag()
}

/// `mod name;` or `mod name { items }`
fn format_module(fmt: &Formatter, item: Module) -> Frag {
    let mut builder = Builder::new(fmt);
    let name = format_name(fmt, item.name().unwrap());
    builder.push(format_item_head(
        fmt,
        item.syntax(),
        item.mod_token().unwrap(),
        name,
        None,
    ));
    match item.item_list() {
        Some(list) => builder.push(format_item_list(fmt, list)),
        None => builder.push_separator(SyntaxKind::SEMICOLON),
    }
    builder.into_frag()
}

/// `impl<T> Trait for Type where ... { items }`, also `unsafe impl`, `impl !Trait for Type`
fn format_impl(fmt: &Formatter, item: Impl) -> Frag {
    let mut builder = Builder::new(fmt);
    let impl_token = item.impl_token().unwrap();
    let impl_start = impl_token.text_range().start();
    let generics = item.generic_param_list();
    let head = format_item_head(fmt, item.syntax(), impl_token, Frag::empty(), generics);
    let trait_ = item.trait_().map(|trait_| {
        let mut builder = Builder::new(fmt);
        // `const` of `impl const Trait`; in front of the generics, it is a qualifier
        if let Some(token) = item
            .const_token()
            .filter(|t| t.text_range().start() > impl_start)
        {
            builder.push_keyword(token);
        }
        if let Some(token) = item.excl_token() {
            builder.push(fmt.token_with_edges(token, Word, Tight));
        }
        builder.push(format_type(fmt, trait_));
        builder.into_frag()
    });
    let mut self_ty = Builder::new(fmt);
    if trait_.is_some() {
        self_ty.push_keyword(item.for_token().unwrap());
    }
    self_ty.push(format_type(fmt, item.self_ty().unwrap()));
    let self_ty = self_ty.into_frag();
    let block_follows = item.where_clause().is_none();
    builder.push(impl_header(fmt, head, trait_, self_ty, block_follows));
    push_where_clause(fmt, &mut builder, item.where_clause(), false);
    builder.push(format_assoc_item_list(fmt, item.assoc_item_list().unwrap()));
    builder.into_frag()
}

/// `impl<T> Trait for Type` (`self_ty` is `for Type`; without a trait, `head` and `self_ty` only).
/// A header that does not fit is broken as rustfmt does (the Style Guide does not say), so that the
/// generics and the generic arguments stay on one line, the rest on indented lines and `{` of
/// `block_follows` on its own line: before `for`, then (dispreferred) after the generics, then after
/// the generics and before `for`; `impl<T> Trait<` / `    A,` / `> for Type {` only when they all
/// overflow or break anyway (it costs one more split)
fn impl_header(
    fmt: &Formatter,
    head: Frag,
    trait_: Option<Frag>,
    self_ty: Frag,
    block_follows: bool,
) -> Frag {
    let rest = Builder::concat(fmt, trait_.clone().into_iter().chain([self_ty.clone()]));
    let joined = Builder::concat(fmt, [head.clone(), rest.clone()]);
    let joined = fmt.keep_together(fmt.keep_together(joined));
    let broken = |first: Frag, lines: &[Frag]| {
        let mut builder = Builder::new(fmt);
        builder.push(fmt.flat(first));
        for line in lines {
            builder.push(fmt.indented_line(line.clone()));
        }
        if block_follows {
            builder.push_newline();
        }
        builder.into_frag()
    };
    let after_generics = fmt.dispreferred(broken(head.clone(), &[rest]));
    let layouts = match trait_ {
        Some(trait_) => {
            let head_and_trait = Builder::concat(fmt, [head.clone(), trait_.clone()]);
            let before_for = broken(head_and_trait, std::slice::from_ref(&self_ty));
            let both = fmt.dispreferred(fmt.dispreferred(broken(head, &[trait_, self_ty])));
            fmt.alt_frag(before_for, fmt.alt_frag(after_generics, both))
        }
        None => after_generics,
    };
    fmt.alt_frag(joined, layouts)
}

/// `trait Name<T>: Bounds where ... { items }`
fn format_trait(fmt: &Formatter, item: Trait) -> Frag {
    if item.eq_token().is_some() {
        return format_unsupported(fmt, item.syntax(), "a trait alias");
    }
    let mut builder = Builder::new(fmt);
    builder.push(format_named_item_head(
        fmt,
        &item,
        item.trait_token().unwrap(),
    ));
    if let Some(colon) = item.colon_token() {
        let block_follows = item.where_clause().is_none();
        let bounds = item.type_bound_list().unwrap();
        builder.push(format_supertraits(fmt, colon, bounds, block_follows));
    }
    push_where_clause(fmt, &mut builder, item.where_clause(), false);
    builder.push(format_assoc_item_list(fmt, item.assoc_item_list().unwrap()));
    builder.into_frag()
}

/// `: Type = value;` of a constant or a static; the value stays on the line of `=` or moves to the
/// next line, indented
fn push_typed_value(
    fmt: &Formatter,
    builder: &mut Builder,
    colon: Option<SyntaxToken>,
    ty: Option<Type>,
    eq: Option<SyntaxToken>,
    value: Option<Expr>,
) {
    if let Some(ty) = ty {
        builder.push_token(colon.unwrap(), Comma);
        builder.push(format_type(fmt, ty));
    }
    if let Some(value) = value {
        builder.push_token(eq.unwrap(), Operator);
        builder.push(fmt.same_or_indented_line(format_expr(fmt, value)));
    }
    builder.push_separator(SyntaxKind::SEMICOLON);
}

/// `const NAME: Type = value;` or `const _: Type = value;`
fn format_const(fmt: &Formatter, item: Const) -> Frag {
    let mut builder = Builder::new(fmt);
    let name = match (item.name(), item.underscore_token()) {
        (Some(name), _) => format_name(fmt, name),
        (None, Some(token)) => fmt.token(token, Word),
        (None, None) => panic!("{item:?}"),
    };
    let keyword = item.const_token().unwrap();
    builder.push(format_item_head(
        fmt,
        item.syntax(),
        keyword,
        name,
        item.generic_param_list(),
    ));
    push_typed_value(
        fmt,
        &mut builder,
        item.colon_token(),
        item.ty(),
        item.eq_token(),
        item.body(),
    );
    builder.into_frag()
}

/// `static NAME: Type = value;` or `static mut NAME: Type = value;`
fn format_static(fmt: &Formatter, item: Static) -> Frag {
    let mut name = Builder::new(fmt);
    if let Some(token) = item.mut_token() {
        name.push_token(token, Word);
    }
    name.push(format_name(fmt, item.name().unwrap()));
    let keyword = item.static_token().unwrap();
    let mut builder = Builder::new(fmt);
    builder.push(format_item_head(
        fmt,
        item.syntax(),
        keyword,
        name.into_frag(),
        None,
    ));
    push_typed_value(
        fmt,
        &mut builder,
        item.colon_token(),
        item.ty(),
        item.eq_token(),
        item.body(),
    );
    builder.into_frag()
}

/// `type Name<T>: Bounds = Type;`; a `where` clause in front of `=` ends with a line break, one
/// after the type precedes `;`. The type stays on the line of `=` or moves to the next line, indented.
fn format_type_alias(fmt: &Formatter, item: TypeAlias) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_named_item_head(
        fmt,
        &item,
        item.type_token().unwrap(),
    ));
    if let Some(colon) = item.colon_token() {
        builder.push_token(colon, Comma);
        builder.push(format_type_bound_list(fmt, item.type_bound_list().unwrap()));
    }
    let where_clause = item.where_clause();
    let where_first = where_clause.as_ref().is_some_and(|clause| {
        item.eq_token()
            .is_none_or(|eq| clause.syntax().text_range().start() < eq.text_range().start())
    });
    if where_first {
        push_where_clause(fmt, &mut builder, where_clause.clone(), item.ty().is_none());
    }
    if let Some(ty) = item.ty() {
        let eq = fmt.token(item.eq_token().unwrap(), Operator);
        let ty = format_type(fmt, ty);
        if where_clause.is_some() && !where_first {
            // With a trailing `where` clause, `= Type` is on its own indented line (Style Guide)
            let mut rhs = Builder::new(fmt);
            rhs.push(eq);
            rhs.push(ty);
            builder.push(fmt.indented_line(rhs.into_frag()));
        } else {
            builder.push(eq);
            builder.push(fmt.same_or_indented_line(ty));
        }
    }
    if !where_first {
        push_where_clause(fmt, &mut builder, where_clause, true);
    }
    builder.push_separator(SyntaxKind::SEMICOLON);
    builder.into_frag()
}

/// An item preceded by its attributes, each on its own line (see `format_skippable_with_attrs`)
pub(super) fn format_item(fmt: &Formatter, item: Item) -> Frag {
    format_skippable_with_attrs(fmt, &item, || match item.clone() {
        Item::Fn(item) => format_fn(fmt, item),
        Item::Struct(item) => format_struct(fmt, item),
        Item::Enum(item) => format_enum(fmt, item),
        Item::Use(item) => format_use(fmt, item),
        Item::Module(item) => format_module(fmt, item),
        Item::Impl(item) => format_impl(fmt, item),
        Item::Trait(item) => format_trait(fmt, item),
        Item::Const(item) => format_const(fmt, item),
        Item::Static(item) => format_static(fmt, item),
        Item::TypeAlias(item) => format_type_alias(fmt, item),
        Item::ExternBlock(item) => format_extern_block(fmt, item),
        Item::MacroCall(item) => format_macro_call(fmt, &item),
        Item::MacroRules(item) => format_macro_rules(fmt, &item),
        item => format_unsupported_kind(fmt, item.syntax()),
    })
}
