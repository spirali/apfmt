//! Patterns

use crate::formatter::{Builder, Formatter, Frag};
use crate::formatter::Edge::{Comma, Operator, Word};
use crate::layout::list::{flat_list, list_items_cast};
use ra_ap_syntax::AstNode;
use ra_ap_syntax::ast::{HasName, IdentPat, Pat, TuplePat, TupleStructPat};
use crate::format::common::{format_name, format_parenthesized};
use super::path::format_path;

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
    let inner = if let [field] = fields.as_slice() {
        // `(a,)` is a tuple, `(a)` is just a parenthesized pattern; the parser guarantees the comma
        let mut builder = Builder::new(fmt);
        builder.push(field.frag.clone());
        if let Some(comma) = field.comma.clone() {
            builder.push_token(comma, Comma);
        }
        builder.to_frag()
    } else {
        flat_list(fmt, &fields, None)
    };
    format_parenthesized(
        fmt,
        pat.l_paren_token().unwrap(),
        inner,
        pat.r_paren_token().unwrap(),
    )
}

fn format_tuple_struct_pat(fmt: &Formatter, pat: TupleStructPat) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_path(fmt, pat.path().unwrap()));
    let fields = list_items_cast(pat.syntax(), |p| format_pat(fmt, p));
    let inner = flat_list(fmt, &fields, None);
    builder.push(format_parenthesized(
        fmt,
        pat.l_paren_token().unwrap(),
        inner,
        pat.r_paren_token().unwrap(),
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
