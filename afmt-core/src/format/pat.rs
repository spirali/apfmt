//! Patterns

use super::attr::format_with_attrs;
use super::expr::{format_range, single_item_separator};
use super::path::{format_name_ref, format_path};
use super::{format_unsupported, format_unsupported_kind};
use crate::comments::{has_comments, trivia_from};
use crate::format::common::format_name;
use crate::formatter::Edge::{Comma, Operator, Tight, Word};
use crate::formatter::{Builder, Formatter, Frag};
use crate::layout::chain::{operator_chain, unindented_infix_chain, unindented_infix_fill};
use crate::layout::head::HeadLayouts;
use crate::layout::list::{
    Delimiters, FlatLimit, ListLayout, TrailingSeparator, delimited_list, list_items,
    list_items_cast,
};
use ra_ap_syntax::ast::{
    HasName, IdentPat, LiteralPat, OrPat, ParenPat, Pat, RangePat, RecordPat, RecordPatField,
    RefPat, RestPat, SlicePat, TuplePat, TupleStructPat,
};
use ra_ap_syntax::{AstNode, SyntaxKind};

/// The layouts of a pattern followed by something on its last line (`= expr`, `in iter`, `if guard`):
/// a broken or-pattern ends on a `| C` continuation line, so it hangs; a broken pattern costs a split
pub(super) fn pat_layouts(fmt: &Formatter, pat: Pat) -> HeadLayouts {
    let or_pat = matches!(pat, Pat::OrPat(_));
    let frag = fmt.keep_together(format_pat(fmt, pat));
    if or_pat {
        HeadLayouts {
            hanging: Some(fmt.multi_line(frag.clone())),
            other: Some(fmt.flat(frag)),
        }
    } else {
        HeadLayouts {
            hanging: None,
            other: Some(frag),
        }
    }
}

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
    builder.into_frag()
}

fn format_tuple_pat(fmt: &Formatter, pat: TuplePat) -> Frag {
    let fields = list_items_cast(pat.syntax(), |p| format_pat(fmt, p));
    let layout = ListLayout::tall(FlatLimit::MaxWidth)
        .trailing_separator(single_item_separator(fields.len()));
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
    builder.into_frag()
}

/// `[a, b, ..]`
fn format_slice_pat(fmt: &Formatter, pat: SlicePat) -> Frag {
    let items = list_items_cast(pat.syntax(), |p| format_pat(fmt, p));
    let delims = Delimiters::brackets_of(pat.syntax());
    delimited_list(fmt, &delims, &items, ListLayout::tall(FlatLimit::MaxWidth))
}

/// `-1`, `"a"`
fn format_literal_pat(fmt: &Formatter, pat: LiteralPat) -> Frag {
    let mut builder = Builder::new(fmt);
    if let Some(token) = pat.minus_token() {
        builder.push_token(token, Tight);
    }
    builder.push_token(pat.literal().unwrap().token(), Word);
    builder.into_frag()
}

/// `&pat` or `&mut pat`
fn format_ref_pat(fmt: &Formatter, pat: RefPat) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_token(pat.amp_token().unwrap(), Tight);
    if let Some(token) = pat.mut_token() {
        builder.push_keyword(token);
    }
    builder.push(format_pat(fmt, pat.pat().unwrap()));
    builder.into_frag()
}

fn format_paren_pat(fmt: &Formatter, pat: ParenPat) -> Frag {
    let inner = format_pat(fmt, pat.pat().unwrap());
    Delimiters::parens_of(pat.syntax()).enclose(fmt, inner)
}

/// `A | B`, broken before every `|` without indentation; a leading `|` is dropped (Style Guide: "Never
/// start a match arm pattern with `|`")
fn format_or_pat(fmt: &Formatter, pat: OrPat) -> Frag {
    // The comments after a dropped `|` would have no place
    let leading = pat
        .syntax()
        .first_token()
        .filter(|t| t.kind() == SyntaxKind::PIPE);
    if leading
        .is_some_and(|pipe| trivia_from(pipe.next_token()).any(|t| t.kind() == SyntaxKind::COMMENT))
    {
        return format_unsupported(fmt, pat.syntax(), "a comment after a leading `|`");
    }
    let (first, rest) = operator_chain(fmt, pat.syntax(), SyntaxKind::PIPE, |node| {
        format_pat(fmt, Pat::cast(node).unwrap())
    });
    // Small alternatives are filled on lines (Style Guide "Match"), unless comments are among them
    if pat.pats().all(|p| is_small_pat(&p, true)) && !has_comments(pat.syntax()) {
        unindented_infix_fill(fmt, first, &rest)
    } else {
        unindented_infix_chain(fmt, first, &rest)
    }
}

/// Whether the pattern is small in the sense of the Style Guide ("Match"): a single token, `&`
/// followed by a small pattern, or (if `allow_tuple`) a tuple of one such pattern (`(a,)`)
fn is_small_pat(pat: &Pat, allow_tuple: bool) -> bool {
    match pat {
        Pat::RefPat(pat) => pat.pat().is_some_and(|p| is_small_pat(&p, allow_tuple)),
        Pat::TuplePat(tuple) if allow_tuple => {
            let mut fields = tuple.fields();
            fields.next().is_some_and(|p| is_small_pat(&p, false))
                && fields.next().is_none()
                && tuple
                    .syntax()
                    .children_with_tokens()
                    .any(|e| e.kind() == SyntaxKind::COMMA)
        }
        pat => {
            let mut tokens = pat
                .syntax()
                .descendants_with_tokens()
                .filter_map(|e| e.into_token())
                .filter(|t| !t.kind().is_trivia());
            tokens.next().is_some() && tokens.next().is_none()
        }
    }
}

/// `name`, `ref mut name` or `name: pat`
fn format_record_pat_field(fmt: &Formatter, field: RecordPatField) -> Frag {
    let mut builder = Builder::new(fmt);
    if let Some(name_ref) = field.name_ref() {
        builder.push(format_name_ref(fmt, name_ref));
        builder.push_token(field.colon_token().unwrap(), Comma);
    }
    builder.push(format_pat(fmt, field.pat().unwrap()));
    builder.into_frag()
}

/// `S { a, b: c, .. }`: on a single line if the fields fit within `struct_lit_width`, otherwise one
/// field per line; `..` is last, without a comma
fn format_record_pat(fmt: &Formatter, pat: RecordPat) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_path(fmt, pat.path().unwrap()));
    let list = pat.record_pat_field_list().unwrap();
    let fields = list_items(list.syntax(), |node| match node.kind() {
        SyntaxKind::RECORD_PAT_FIELD => {
            let field = RecordPatField::cast(node).unwrap();
            // `#[cfg(x)]` / `field,`, as in struct literals
            Some(format_with_attrs(fmt, &field, false, || {
                format_record_pat_field(fmt, field.clone())
            }))
        }
        SyntaxKind::REST_PAT => Some(format_rest_pat(fmt, RestPat::cast(node).unwrap())),
        _ => None,
    });
    let trailing = if list.rest_pat().is_some() {
        TrailingSeparator::Omitted
    } else {
        TrailingSeparator::IfVertical
    };
    let layout = ListLayout::tall(FlatLimit::Within(fmt.config().struct_lit_width))
        .trailing_separator(trailing);
    let delims = Delimiters::braces_of(list.syntax());
    builder.push(delimited_list(fmt, &delims, &fields, layout));
    builder.into_frag()
}

/// `..`
fn format_rest_pat(fmt: &Formatter, pat: RestPat) -> Frag {
    fmt.token(pat.dotdot_token().unwrap(), Word)
}

fn format_range_pat(fmt: &Formatter, pat: RangePat) -> Frag {
    format_range(fmt, &pat, |p| format_pat(fmt, p))
}

pub(super) fn format_pat(fmt: &Formatter, pat: Pat) -> Frag {
    match pat {
        Pat::IdentPat(pat) => format_ident_pat(fmt, pat),
        Pat::WildcardPat(pat) => fmt.token(pat.underscore_token().unwrap(), Word),
        Pat::TuplePat(pat) => format_tuple_pat(fmt, pat),
        Pat::TupleStructPat(pat) => format_tuple_struct_pat(fmt, pat),
        Pat::PathPat(pat) => format_path(fmt, pat.path().unwrap()),
        Pat::LiteralPat(pat) => format_literal_pat(fmt, pat),
        Pat::RefPat(pat) => format_ref_pat(fmt, pat),
        Pat::ParenPat(pat) => format_paren_pat(fmt, pat),
        Pat::OrPat(pat) => format_or_pat(fmt, pat),
        Pat::SlicePat(pat) => format_slice_pat(fmt, pat),
        Pat::RecordPat(pat) => format_record_pat(fmt, pat),
        Pat::RestPat(pat) => format_rest_pat(fmt, pat),
        Pat::RangePat(pat) => format_range_pat(fmt, pat),
        pat => format_unsupported_kind(fmt, pat.syntax()),
    }
}
