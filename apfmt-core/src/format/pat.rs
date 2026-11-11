//! Patterns

use super::attr::format_with_attrs;
use super::expr::format_range;
use super::expr_list::single_item_separator;
use super::path::{format_name_ref, format_path};
use super::{format_unsupported, format_unsupported_kind};
use crate::comments::{comment_follows, has_comments, is_code_kind};
use crate::format::common::format_name;
use crate::formatter::Edge::{Comma, Operator, Tight, Word};
use crate::formatter::{Builder, Formatter, Frag, NextLine};
use crate::layout::chain::{operator_chain, unindented_infix_chain, unindented_infix_fill};
use crate::layout::head::HeadLayouts;
use crate::layout::list::{
    Delimiters, FlatLimit, Hug, ListLayout, TrailingSeparator, delimited_list, hug_last_item,
    list_items, list_items_cast,
};
use ra_ap_syntax::ast::{
    HasName, IdentPat, LiteralPat, OrPat, ParenPat, Pat, RangePat, RecordPat, RecordPatField,
    RefPat, RestPat, SlicePat, TuplePat, TupleStructPat,
};
use ra_ap_syntax::{AstNode, SyntaxKind, SyntaxNode};

/// A pattern followed by more on its last line (`= expr`, `in iter`): a broken or-pattern hangs; a
/// broken pattern costs a split
pub(super) fn pat_layouts(fmt: &Formatter, pat: Pat) -> HeadLayouts {
    let or_pat = matches!(pat, Pat::OrPat(_));
    let frag = fmt.keep_together(format_pat(fmt, pat));
    if or_pat {
        HeadLayouts::hanging_when_broken(fmt, frag)
    } else {
        HeadLayouts::other(frag)
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

/// A single pattern in parentheses that may overflow: a struct, tuple struct or tuple pattern, also
/// after `&`, as in rustfmt (`can_be_overflowed_pat`); not a slice pattern
fn is_combinable_pat(pat: &Pat) -> bool {
    match pat {
        Pat::RecordPat(_) | Pat::TupleStructPat(_) | Pat::TuplePat(_) => true,
        Pat::RefPat(pat) => pat.pat().is_some_and(|p| is_combinable_pat(&p)),
        _ => false,
    }
}

/// A single combinable pattern, as a call argument does
fn last_pat_hug(pats: &[Pat]) -> Hug {
    match pats {
        [pat] if is_combinable_pat(pat) => Hug::Allowed,
        _ => Hug::Never,
    }
}

/// The last pattern may hug the parentheses (see `last_pat_hug`)
fn format_pat_list(fmt: &Formatter, node: &SyntaxNode, layout: ListLayout) -> Frag {
    let pats = node.children().filter_map(Pat::cast).collect::<Vec<_>>();
    let hug = last_pat_hug(&pats);
    let mut items = list_items_cast(node, |p| format_pat(fmt, p));
    hug_last_item(&mut items, hug);
    delimited_list(fmt, &Delimiters::parens_of(node), &items, layout)
}

fn format_tuple_pat(fmt: &Formatter, pat: TuplePat) -> Frag {
    let mut fields = pat.fields();
    // `(..)` is a tuple pattern already, unlike `(a)`
    let separator = match (fields.next(), fields.next()) {
        (Some(Pat::RestPat(_)), None) => TrailingSeparator::IfVertical,
        _ => single_item_separator(pat.fields().count()),
    };
    let layout = ListLayout::tall(FlatLimit::MaxWidth).trailing_separator(separator);
    format_pat_list(fmt, pat.syntax(), layout)
}

fn format_tuple_struct_pat(fmt: &Formatter, pat: TupleStructPat) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_path(fmt, pat.path().unwrap()));
    let layout = ListLayout::tall(FlatLimit::MaxWidth);
    builder.push(format_pat_list(fmt, pat.syntax(), layout));
    builder.into_frag()
}

/// `[a, b, ..]`, on a single line within `array_width` (as rustfmt), otherwise one per line
fn format_slice_pat(fmt: &Formatter, pat: SlicePat) -> Frag {
    let items = list_items_cast(pat.syntax(), |p| format_pat(fmt, p));
    let delims = Delimiters::brackets_of(pat.syntax());
    let layout = ListLayout::tall(FlatLimit::Within(fmt.config().array_width));
    delimited_list(fmt, &delims, &items, layout)
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

/// `A | B`, broken before every `|` without indentation; a leading `|` is dropped (Style Guide)
fn format_or_pat(fmt: &Formatter, pat: OrPat) -> Frag {
    // The comments after a dropped `|` would have no place
    let leading = pat
        .syntax()
        .first_token()
        .filter(|t| t.kind() == SyntaxKind::PIPE);
    if leading.is_some_and(|pipe| comment_follows(&pipe)) {
        return format_unsupported(fmt, pat.syntax(), "a comment after a leading `|`");
    }
    let (first, rest) = operator_chain(fmt, pat.syntax(), SyntaxKind::PIPE, |node| {
        format_pat(fmt, Pat::cast(node).unwrap())
    });
    // Small alternatives are filled on lines (Style Guide "Match"), unless commented
    if pat.pats().all(|p| is_small_pat(&p, true)) && !has_comments(pat.syntax()) {
        unindented_infix_fill(fmt, first, &rest)
    } else {
        unindented_infix_chain(fmt, first, &rest)
    }
}

/// Style Guide ("Match"): a single token, `&` and a small pattern, or (`allow_tuple`) `(a,)`
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
                .filter(|t| is_code_kind(t.kind()));
            tokens.next().is_some() && tokens.next().is_none()
        }
    }
}

/// `name`, `ref mut name` or `name: pat`, the pattern on the line of the name or the next
/// (RhsBreak, as rustfmt)
fn format_record_pat_field(fmt: &Formatter, field: RecordPatField) -> Frag {
    let pat = format_pat(fmt, field.pat().unwrap());
    let Some(name_ref) = field.name_ref() else {
        return pat;
    };
    let mut builder = Builder::new(fmt);
    builder.push(format_name_ref(fmt, name_ref));
    builder.push_token(field.colon_token().unwrap(), Comma);
    builder.push(fmt.same_or_indented_line(pat, NextLine::InOrder));
    builder.into_frag()
}

/// `S { a, b: c, .. }`: on a single line within `struct_lit_width`, otherwise one field per line
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
    // Blank lines between the fields are kept, as in a struct literal (as rustfmt)
    let layout = ListLayout::tall(FlatLimit::Within(fmt.config().struct_lit_width))
        .trailing_separator(trailing)
        .keep_blank_lines();
    let delims = Delimiters::braces_of(list.syntax());
    builder.push(delimited_list(fmt, &delims, &fields, layout));
    builder.into_frag()
}

fn format_rest_pat(fmt: &Formatter, pat: RestPat) -> Frag {
    fmt.token(pat.dotdot_token().unwrap(), Word)
}

/// Also the edition 2015 `a...b`, kept as it is (as rustfmt): ra_ap_syntax sees neither its `...`
/// nor its end
fn format_range_pat(fmt: &Formatter, pat: RangePat) -> Frag {
    let dots = pat
        .syntax()
        .children_with_tokens()
        .find(|it| it.kind() == SyntaxKind::DOT3);
    let Some(dots) = dots.and_then(|it| it.into_token()) else {
        return format_range(fmt, &pat, |p| format_pat(fmt, p));
    };
    let mut bounds = pat.syntax().children().filter_map(Pat::cast);
    let mut builder = Builder::new(fmt);
    builder.push(format_pat(fmt, bounds.next().unwrap()));
    match bounds.next() {
        Some(end) => {
            builder.push_token(dots, Tight);
            builder.push(format_pat(fmt, end));
        }
        None => builder.push(fmt.postfix_token(dots)),
    }
    builder.into_frag()
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

#[cfg(test)]
mod tests {
    use super::{Hug, last_pat_hug};
    use ra_ap_syntax::ast::Pat;
    use ra_ap_syntax::{AstNode, Edition, SourceFile, SyntaxKind};

    fn hug(pat: &str) -> Hug {
        let source = format!("fn f() {{ let {pat} = x; }}");
        let parse = SourceFile::parse(&source, Edition::Edition2024);
        let list = (parse.syntax_node().descendants())
            .find(|n| {
                matches!(
                    n.kind(),
                    SyntaxKind::TUPLE_PAT | SyntaxKind::TUPLE_STRUCT_PAT
                )
            })
            .unwrap();
        let pats = list.children().filter_map(Pat::cast).collect::<Vec<_>>();
        last_pat_hug(&pats)
    }

    #[test]
    fn last_pat_hugs() {
        use Hug::{Allowed, Never};
        for (pat, expected) in [
            ("Some(a)", Never),
            ("Some(_)", Never),
            ("Some(Foo { a, b })", Allowed),
            ("Some(Foo(a, b))", Allowed),
            ("Some((a, b))", Allowed),
            ("Some(&Foo { a })", Allowed),
            ("Some(&mut (a, b))", Allowed),
            ("Some([a, b])", Never),
            ("Some(a | b)", Never),
            ("Foo(a, Bar { b })", Never),
            ("(Foo { a },)", Allowed),
            ("(a, b)", Never),
        ] {
            assert_eq!(hug(pat), expected, "{pat}");
        }
    }
}
