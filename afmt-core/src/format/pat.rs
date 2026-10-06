//! Patterns

use super::path::format_path;
use crate::format::common::format_name;
use crate::formatter::Edge::{Operator, Word};
use crate::formatter::{Builder, Formatter, Frag};
use crate::layout::list::{
    Delimiters, FlatLimit, ListLayout, TrailingSeparator, delimited_list, list_items_cast,
};
use ra_ap_syntax::AstNode;
use ra_ap_syntax::ast::{HasName, IdentPat, Pat, TuplePat, TupleStructPat};

fn format_ident_pat(fmt: &Formatter, pat: IdentPat) -> Frag {
    let mut builder = Builder::new(fmt);
    if let Some(token) = pat.ref_token() {
        builder.push_token(token, Word);
    }
    if let Some(token) = pat.mut_token() {
        builder.push_token(token, Word);
    }
    builder.push(format_name(fmt, pat.name().unwrap()));
    if let Some(token) = pat.at_token() {
        builder.push_token(token, Operator);
        builder.push(format_pat(fmt, pat.pat().unwrap()));
    }
    builder.to_frag()
}

fn format_tuple_pat(fmt: &Formatter, pat: TuplePat) -> Frag {
    let fields = list_items_cast(pat.syntax(), |p| format_pat(fmt, p));
    // `(a,)` is a tuple, `(a)` is just a parenthesized pattern
    let trailing = if fields.len() == 1 {
        TrailingSeparator::Required
    } else {
        TrailingSeparator::IfVertical
    };
    let layout = ListLayout::tall(FlatLimit::MaxWidth).trailing_separator(trailing);
    delimited_list(fmt, &Delimiters::parens_of(pat.syntax()), &fields, layout)
}

fn format_tuple_struct_pat(fmt: &Formatter, pat: TupleStructPat) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_path(fmt, pat.path().unwrap()));
    let fields = list_items_cast(pat.syntax(), |p| format_pat(fmt, p));
    let delims = Delimiters::parens_of(pat.syntax());
    builder.push(delimited_list(
        fmt,
        &delims,
        &fields,
        ListLayout::tall(FlatLimit::MaxWidth),
    ));
    builder.to_frag()
}

pub(super) fn format_pat(fmt: &Formatter, pat: Pat) -> Frag {
    match pat {
        Pat::IdentPat(pat) => format_ident_pat(fmt, pat),
        Pat::WildcardPat(pat) => fmt.token(pat.underscore_token().unwrap(), Word),
        Pat::TuplePat(pat) => format_tuple_pat(fmt, pat),
        Pat::TupleStructPat(pat) => format_tuple_struct_pat(fmt, pat),
        x => {
            panic!("{x:?}")
        }
    }
}
