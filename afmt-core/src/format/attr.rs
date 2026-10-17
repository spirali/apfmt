//! Attributes: `#[derive(Debug)]`, `#![allow(x)]`, `#[cfg(test)]`, `#[doc = "..."]`

use super::expr::format_expr;
use super::path::format_path;
use crate::comments::has_comments;
use crate::formatter::Edge::{Operator, Tight, Word};
use crate::formatter::{Builder, Formatter, Frag};
use crate::layout::list::{
    Delimiters, FlatLimit, ListItem, ListLayout, delimited_list, hug_single_item, list_items,
};
use ra_ap_syntax::ast::{
    Attr, CfgAtom, CfgAttrMeta, CfgPredicate, HasAttrs, KeyValueMeta, Meta, TokenTree,
    TokenTreeMeta, UnsafeMeta,
};
use ra_ap_syntax::{AstNode, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

/// `#[meta]` or `#![meta]`
pub(super) fn format_attr(fmt: &Formatter, attr: Attr) -> Frag {
    fmt.mark_placed(attr.syntax());
    let mut builder = Builder::new(fmt);
    // Spaced as a word in front, e.g. after a block comment: `/* c */ #[attr]`
    builder.push(fmt.token_with_edges(attr.pound_token().unwrap(), Word, Tight));
    if let Some(token) = attr.excl_token() {
        builder.push_token(token, Tight);
    }
    let meta = format_meta(fmt, attr.meta().unwrap());
    builder.push(Delimiters::brackets_of(attr.syntax()).enclose(fmt, meta));
    builder.into_frag()
}

/// `node` preceded by its outer attributes; `format` formats the rest of
/// it. With `inline`, the attributes stay on the line of the node (`#[cfg(x)] a: u32`), otherwise
/// each is on its own line (`#[derive(Debug)]` / `struct S;`).
pub(super) fn format_with_attrs(
    fmt: &Formatter,
    node: &impl HasAttrs,
    inline: bool,
    format: impl FnOnce() -> Frag,
) -> Frag {
    let attrs = node
        .attrs()
        .map(|attr| format_attr(fmt, attr))
        .collect::<Vec<_>>();
    attach_attrs(fmt, attrs, inline, format())
}

/// Checks in debug builds that the attributes of `node` are placed (by `format_attr`) before the
/// rest of it is formatted: code formatting an expression that may have attributes goes through
/// `format_expr` or `format_expr_at`, which place them or copy them with the expression
pub(super) fn debug_assert_attrs_placed(fmt: &Formatter, node: &impl HasAttrs) {
    debug_assert!(
        node.attrs().all(|attr| fmt.was_placed(attr.syntax())),
        "attributes not placed: {}",
        node.syntax()
    );
}

/// Whether the attribute is `#[rustfmt::skip]`
fn is_rustfmt_skip(attr: &Attr) -> bool {
    matches!(attr.meta(), Some(Meta::PathMeta(meta)) if meta.path().is_some_and(|p| p.syntax().text() == "rustfmt::skip"))
}

/// `format_with_attrs` with each attribute on its own line, for a node that `#[rustfmt::skip]`
/// keeps as it is in the source, from its first attribute on (an item, a statement)
pub(super) fn format_skippable_with_attrs(
    fmt: &Formatter,
    node: &impl HasAttrs,
    format: impl FnOnce() -> Frag,
) -> Frag {
    let Some(first_attr) = node.attrs().next() else {
        return format();
    };
    if node.attrs().any(|attr| is_rustfmt_skip(&attr)) {
        let first = first_attr.syntax().first_token().unwrap();
        let last = node.syntax().last_token().unwrap();
        return fmt.verbatim_range(&first, &last, Word, Word);
    }
    format_with_attrs(fmt, node, false, format)
}

/// `rest` preceded by the formatted attributes `attrs`, see `format_with_attrs`
pub(super) fn attach_attrs(
    fmt: &Formatter,
    attrs: impl IntoIterator<Item = Frag>,
    inline: bool,
    rest: Frag,
) -> Frag {
    let mut builder = Builder::new(fmt);
    if inline {
        for attr in attrs {
            builder.push(fmt.followed_by_space(attr));
        }
        builder.push(rest);
    } else {
        for frag in attrs.into_iter().chain([rest]) {
            builder.push_on_new_line(frag);
        }
    }
    builder.into_frag()
}

fn format_meta(fmt: &Formatter, meta: Meta) -> Frag {
    match meta {
        Meta::PathMeta(meta) => format_path(fmt, meta.path().unwrap()),
        Meta::KeyValueMeta(meta) => format_key_value_meta(fmt, meta),
        Meta::TokenTreeMeta(meta) => format_token_tree_meta(fmt, meta),
        Meta::CfgMeta(meta) => format_cfg_list(fmt, meta.cfg_token().unwrap(), meta.syntax()),
        Meta::CfgAttrMeta(meta) => format_cfg_attr_meta(fmt, meta),
        Meta::UnsafeMeta(meta) => format_unsafe_meta(fmt, meta),
    }
}

/// `path = expr`, e.g. `doc = "text"`
fn format_key_value_meta(fmt: &Formatter, meta: KeyValueMeta) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_path(fmt, meta.path().unwrap()));
    builder.push_token(meta.eq_token().unwrap(), Operator);
    builder.push(format_expr(fmt, meta.expr().unwrap()));
    builder.into_frag()
}

/// The comma-separated `items` of an attribute between the parentheses of `node`: on a single line
/// if they fit within `attr_fn_like_width`, otherwise one per line, laid out as call arguments, or
/// filled on lines with `fill`. A single item that is a list itself (`nested`) may overflow:
/// `cfg(all(` / `...` / `))`; as in rustfmt, its own items are measured, not the item, so the outer
/// list is on a single line whenever the inner one is. Filled derives are on a single line whenever
/// it fits within `max_width`, as in rustfmt.
fn format_meta_list(
    fmt: &Formatter,
    node: &SyntaxNode,
    mut items: Vec<ListItem>,
    nested: bool,
    fill: bool,
) -> Frag {
    if nested {
        hug_single_item(&mut items);
    }
    let delims = Delimiters::parens_of(node);
    let limit = if nested || fill {
        FlatLimit::MaxWidth
    } else {
        FlatLimit::Within(fmt.config().attr_fn_like_width)
    };
    let layout = ListLayout::tall(limit).fill_if(fill);
    delimited_list(fmt, &delims, &items, layout)
}

/// Whether the node in a meta list is a list itself: `all(a, b)`, `derive(A)`
fn is_nested_meta_list(node: &SyntaxNode) -> bool {
    matches!(
        node.kind(),
        SyntaxKind::CFG_COMPOSITE
            | SyntaxKind::CFG_META
            | SyntaxKind::CFG_ATTR_META
            | SyntaxKind::TOKEN_TREE_META
    )
}

/// The predicates of a `cfg` list and whether the only one is a list itself
fn cfg_predicate_items(fmt: &Formatter, node: &SyntaxNode) -> (Vec<ListItem>, bool) {
    let items = list_items(node, |child| {
        CfgPredicate::cast(child).map(|p| format_cfg_predicate(fmt, p))
    });
    let nested = node.children().all(|child| is_nested_meta_list(&child));
    (items, nested)
}

/// `cfg(a, b)`, `any(a, b)`, `all(a, b)` or `not(a)`: `keyword` and the predicates of `node`
fn format_cfg_list(fmt: &Formatter, keyword: SyntaxToken, node: &SyntaxNode) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_token(keyword, Word);
    let (items, nested) = cfg_predicate_items(fmt, node);
    builder.push(format_meta_list(fmt, node, items, nested, false));
    builder.into_frag()
}

/// `cfg_attr(predicate, meta, ...)`
fn format_cfg_attr_meta(fmt: &Formatter, meta: CfgAttrMeta) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_token(meta.cfg_attr_token().unwrap(), Word);
    let items = list_items(meta.syntax(), |node| {
        match CfgPredicate::cast(node.clone()) {
            Some(predicate) => Some(format_cfg_predicate(fmt, predicate)),
            None => Meta::cast(node).map(|m| format_meta(fmt, m)),
        }
    });
    builder.push(format_meta_list(fmt, meta.syntax(), items, false, false));
    builder.into_frag()
}

/// `unsafe(meta)`
fn format_unsafe_meta(fmt: &Formatter, meta: UnsafeMeta) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_token(meta.unsafe_token().unwrap(), Word);
    let inner = format_meta(fmt, meta.meta().unwrap());
    builder.push(Delimiters::parens_of(meta.syntax()).enclose(fmt, inner));
    builder.into_frag()
}

fn format_cfg_predicate(fmt: &Formatter, predicate: CfgPredicate) -> Frag {
    match predicate {
        CfgPredicate::CfgAtom(atom) => format_cfg_atom(fmt, atom),
        CfgPredicate::CfgComposite(composite) => {
            format_cfg_list(fmt, composite.keyword().unwrap(), composite.syntax())
        }
    }
}

/// `test`, `feature = "x"`, `true` or `false`
fn format_cfg_atom(fmt: &Formatter, atom: CfgAtom) -> Frag {
    let mut builder = Builder::new(fmt);
    let name = atom
        .ident_token()
        .or(atom.true_token())
        .or(atom.false_token());
    builder.push_token(name.unwrap(), Word);
    if let Some(eq_token) = atom.eq_token() {
        builder.push_token(eq_token, Operator);
        builder.push_token(atom.string_token().unwrap(), Word);
    }
    builder.into_frag()
}

/// `path(token tree)`, e.g. `derive(Debug, Clone)`
fn format_token_tree_meta(fmt: &Formatter, meta: TokenTreeMeta) -> Frag {
    let mut builder = Builder::new(fmt);
    let path = meta.path().unwrap();
    let derive = path.syntax().text() == "derive";
    builder.push(format_path(fmt, path));
    builder.push(format_token_list(fmt, meta.token_tree().unwrap(), derive));
    builder.into_frag()
}

/// A token tree as a list of meta items, `(a, b = "x", c(d))`, as `format_meta_list` does; a token tree
/// that is not such a list, or that contains comments, is kept as it is in the source. With
/// `fill_paths`, a broken list of paths is filled on lines: the derives of `derive(...)`, an explicit
/// exception to the Style Guide, as in rustfmt.
fn format_token_list(fmt: &Formatter, tree: TokenTree, fill_paths: bool) -> Frag {
    let Some(segments) = meta_segments(&tree) else {
        return Delimiters::of_token_tree(&tree).format_reindented(fmt);
    };
    // A single `a(...)`: a path followed by a token tree
    let nested = match &segments[..] {
        [(segment, _)] => segment
            .last()
            .is_some_and(|e| e.kind() == SyntaxKind::TOKEN_TREE),
        _ => false,
    };
    let fill = fill_paths && segments.iter().all(|(segment, _)| is_path_segment(segment));
    let items = segments
        .into_iter()
        .map(|(segment, comma)| ListItem {
            comma,
            ..ListItem::new(format_meta_segment(fmt, segment))
        })
        .collect::<Vec<_>>();
    format_meta_list(fmt, tree.syntax(), items, nested, fill)
}

/// The tokens of a meta item in a token tree, without trivia, and the comma following it
type MetaSegment = (Vec<SyntaxElement>, Option<SyntaxToken>);

/// The comma-separated meta items of a parenthesized token tree, if all of them are meta items (see
/// `is_meta_segment`) and the tree contains no comments
fn meta_segments(tree: &TokenTree) -> Option<Vec<MetaSegment>> {
    if tree.l_paren_token().is_none() || has_comments(tree.syntax()) {
        return None;
    }
    let mut segments: Vec<MetaSegment> = Vec::new();
    let mut current = Vec::new();
    let children = tree.syntax().children_with_tokens().collect::<Vec<_>>();
    // Without the delimiters
    for element in &children[1..children.len() - 1] {
        match element.kind() {
            kind if kind.is_trivia() => {}
            SyntaxKind::COMMA => {
                let comma = element.clone().into_token();
                segments.push((std::mem::take(&mut current), comma));
            }
            _ => current.push(element.clone()),
        }
    }
    if !current.is_empty() {
        segments.push((current, None));
    }
    segments
        .iter()
        .all(|(segment, _)| is_meta_segment(segment))
        .then_some(segments)
}

/// A literal, or a path (`a`, `a::b`) optionally followed by `= literal` or by a token tree
fn is_meta_segment(segment: &[SyntaxElement]) -> bool {
    if let [literal] = segment
        && literal.kind().is_literal()
    {
        return true;
    }
    let mut rest = segment;
    loop {
        let [name, tail @ ..] = rest else {
            return false;
        };
        if name.kind() != SyntaxKind::IDENT {
            return false;
        }
        rest = tail;
        match rest {
            [colon1, colon2, tail @ ..]
                if colon1.kind() == SyntaxKind::COLON && colon2.kind() == SyntaxKind::COLON =>
            {
                rest = tail
            }
            _ => break,
        }
    }
    match rest {
        [] => true,
        [eq, literal] => eq.kind() == SyntaxKind::EQ && literal.kind().is_literal(),
        [tree] => tree.kind() == SyntaxKind::TOKEN_TREE,
        _ => false,
    }
}

/// Whether the meta item is a path: `Debug`, `serde::Serialize`
fn is_path_segment(segment: &[SyntaxElement]) -> bool {
    segment
        .iter()
        .all(|e| matches!(e.kind(), SyntaxKind::IDENT | SyntaxKind::COLON))
}

/// A meta item checked by `is_meta_segment`: `a::b`, `a = "x"`, `a(...)`
fn format_meta_segment(fmt: &Formatter, segment: Vec<SyntaxElement>) -> Frag {
    let mut builder = Builder::new(fmt);
    for element in segment {
        match element {
            SyntaxElement::Node(node) => builder.push(format_token_list(
                fmt,
                TokenTree::cast(node).unwrap(),
                false,
            )),
            SyntaxElement::Token(token) => {
                let edge = match token.kind() {
                    SyntaxKind::COLON => Tight,
                    SyntaxKind::EQ => Operator,
                    _ => Word,
                };
                builder.push_token(token, edge);
            }
        }
    }
    builder.into_frag()
}
