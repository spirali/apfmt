//! Attributes: `#[derive(Debug)]`, `#![allow(x)]`, `#[cfg(test)]`, `#[doc = "..."]`

use super::expr::format_expr;
use super::path::format_path;
use crate::comments::{comment_follows, has_comments, is_code_kind, starts_on_line_of_previous_token};
use crate::formatter::Edge::{Operator, Tight, Word};
use crate::formatter::{Builder, Formatter, Frag};
use crate::layout::list::{
    Delimiters, FlatLimit, Hug, ListItem, ListLayout, delimited_list, hug_last_item, list_items,
};
use crate::syntax_utils::has_rustfmt_skip;
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
    // A blank line in front of an attribute of the same kind is dropped, as in rustfmt
    let next_attr = attr.syntax().next_sibling().and_then(Attr::cast);
    if next_attr.is_some_and(|next| next.excl_token().is_some() == attr.excl_token().is_some())
        && !builder.cursor().is_some_and(comment_follows)
    {
        builder.flush_trivia();
    }
    builder.into_frag()
}

/// `node` preceded by its outer attributes, each on its own line; with `inline`, a single one may
/// stay on the line of the node (`#[cfg(x)] a: u32`), as rustfmt does for fields and parameters
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

/// Debug check that the attributes are placed before the rest is formatted (an expression goes
/// through `format_expr` or `format_expr_at`)
pub(super) fn debug_assert_attrs_placed(fmt: &Formatter, node: &impl HasAttrs) {
    debug_assert!(
        node.attrs().all(|attr| fmt.was_placed(attr.syntax())),
        "attributes not placed: {}",
        node.syntax()
    );
}

/// For a node that `#[rustfmt::skip]` keeps as in the source (an item, a statement, a field, a
/// match arm)
pub(super) fn format_skippable_with_attrs(
    fmt: &Formatter,
    node: &impl HasAttrs,
    format: impl FnOnce() -> Frag,
) -> Frag {
    skipped_verbatim(fmt, node).unwrap_or_else(|| format_with_attrs(fmt, node, false, format))
}

/// The node with its attributes as in the source, if one of them is `#[rustfmt::skip]`
pub(super) fn skipped_verbatim(fmt: &Formatter, node: &impl HasAttrs) -> Option<Frag> {
    if !has_rustfmt_skip(node) {
        return None;
    }
    let first = node.attrs().next().unwrap().syntax().first_token().unwrap();
    let last = node.syntax().last_token().unwrap();
    Some(fmt.verbatim_range(&first, &last, Word, Word))
}

pub(super) fn attach_attrs(
    fmt: &Formatter,
    attrs: impl IntoIterator<Item = Frag>,
    inline: bool,
    rest: Frag,
) -> Frag {
    let attrs = attrs.into_iter().collect::<Vec<_>>();
    let mut own_lines = Builder::new(fmt);
    for frag in attrs.iter().cloned().chain([rest.clone()]) {
        own_lines.push_on_new_line(frag);
    }
    let own_lines = own_lines.into_frag();
    // A single attribute on one line in front of the node, unless a comment follows it
    let inline_attr = match attrs.as_slice() {
        [attr] if inline && !attr.comment_follows() => fmt.flat_if_possible(attr.clone()),
        _ => None,
    };
    match inline_attr {
        Some(attr) => {
            let same_line = Builder::concat(fmt, [fmt.followed_by_space(attr), rest]);
            fmt.alt_frag(same_line, own_lines)
        }
        None => own_lines,
    }
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

/// The items of an attribute: on a single line within `attr_fn_like_width`, otherwise one per line
/// or filled with `fill`; the last item may hug the list (`cfg(all(` / `...` / `))`), as in rustfmt
fn format_meta_list(
    fmt: &Formatter,
    node: &SyntaxNode,
    mut items: Vec<ListItem>,
    hug: Hug,
    fill: bool,
) -> Frag {
    hug_last_item(&mut items, hug);
    let delims = Delimiters::parens_of(node);
    let limit = if hug != Hug::Never || fill {
        FlatLimit::MaxWidth
    } else {
        FlatLimit::of_arguments(fmt, &items, hug, fmt.config().attr_fn_like_width)
    };
    let layout = ListLayout::tall(limit).fill_if(fill);
    delimited_list(fmt, &delims, &items, layout)
}

/// E.g. `all(a, b)`, `derive(A)`
fn is_nested_meta_list(node: &SyntaxNode) -> bool {
    matches!(
        node.kind(),
        SyntaxKind::CFG_COMPOSITE
            | SyntaxKind::CFG_META
            | SyntaxKind::CFG_ATTR_META
            | SyntaxKind::TOKEN_TREE_META
    )
}

/// A single predicate that is a list itself
fn cfg_list_hug(node: &SyntaxNode) -> Hug {
    let predicates = node
        .children()
        .filter_map(CfgPredicate::cast)
        .collect::<Vec<_>>();
    match predicates.as_slice() {
        [single] if is_nested_meta_list(single.syntax()) => Hug::Allowed,
        _ => Hug::Never,
    }
}

/// `cfg(a, b)`, `any(a, b)`, `all(a, b)` or `not(a)`: `keyword` and the predicates of `node`
fn format_cfg_list(fmt: &Formatter, keyword: SyntaxToken, node: &SyntaxNode) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_token(keyword, Word);
    let items = list_items(node, |child| {
        CfgPredicate::cast(child).map(|p| format_cfg_predicate(fmt, p))
    });
    builder.push(format_meta_list(
        fmt,
        node,
        items,
        cfg_list_hug(node),
        false,
    ));
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
    builder.push(format_meta_list(
        fmt,
        meta.syntax(),
        items,
        Hug::Never,
        false,
    ));
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

/// A token tree as a list of meta items, `(a, b = "x", c(d))`; otherwise, or with comments, kept as
/// in the source. With `fill_paths`, broken derives are filled (as rustfmt).
fn format_token_list(fmt: &Formatter, tree: TokenTree, fill_paths: bool) -> Frag {
    let Some(segments) = meta_segments(&tree) else {
        return Delimiters::of_token_tree(&tree).format_reindented(fmt);
    };
    let hug = token_list_hug(&segments, fmt.config().max_width);
    let fill = fill_paths && segments.iter().all(|(segment, _)| is_path_segment(segment));
    let items = segments
        .into_iter()
        .map(|(segment, comma)| ListItem {
            comma,
            ..ListItem::new(format_meta_segment(fmt, segment))
        })
        .collect::<Vec<_>>();
    format_meta_list(fmt, tree.syntax(), items, hug, fill)
}

/// A single `a(...)`, or a single string spanning lines that hugs as in the source
fn token_list_hug(segments: &[MetaSegment], max_width: u32) -> Hug {
    match segments {
        [(segment, _)]
            if segment
                .last()
                .is_some_and(|e| e.kind() == SyntaxKind::TOKEN_TREE) =>
        {
            Hug::Allowed
        }
        [(segment, _)] if is_hugging_string_segment(segment) => {
            let text = segment.last().map(|e| e.to_string()).unwrap_or_default();
            Hug::of_string(&text, max_width)
        }
        _ => Hug::Never,
    }
}

/// `"..."` or `name = "..."` spanning lines, starting on the line of `(` (`is_hugging_string`)
fn is_hugging_string_segment(segment: &[SyntaxElement]) -> bool {
    let is_multi_line_string = segment.last().and_then(|e| e.as_token()).is_some_and(|t| {
        matches!(
            t.kind(),
            SyntaxKind::STRING | SyntaxKind::BYTE_STRING | SyntaxKind::C_STRING
        ) && t.text().contains('\n')
    });
    is_multi_line_string
        && (segment.first().and_then(|e| e.as_token())).is_some_and(starts_on_line_of_previous_token)
}

/// Without trivia, with the comma following it
type MetaSegment = (Vec<SyntaxElement>, Option<SyntaxToken>);

/// If all of them are meta items and the tree contains no comments
fn meta_segments(tree: &TokenTree) -> Option<Vec<MetaSegment>> {
    if tree.l_paren_token().is_none() || has_comments(tree.syntax()) {
        return None;
    }
    let mut segments: Vec<MetaSegment> = Vec::new();
    let mut current = Vec::new();
    let children = tree.syntax().children_with_tokens().collect::<Vec<_>>();
    for element in &children[1..children.len() - 1] {
        match element.kind() {
            kind if !is_code_kind(kind) => {}
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

#[cfg(test)]
mod tests {
    use super::{Hug, cfg_list_hug, meta_segments, token_list_hug};
    use ra_ap_syntax::ast::TokenTree;
    use ra_ap_syntax::{AstNode, Edition, SourceFile, SyntaxKind};

    fn hug(attr: &str) -> Hug {
        let source = format!("{attr} fn f() {{}}");
        let parse = SourceFile::parse(&source, Edition::Edition2024);
        let mut nodes = parse.syntax_node().descendants();
        let list = nodes
            .find(|n| matches!(n.kind(), SyntaxKind::CFG_COMPOSITE | SyntaxKind::TOKEN_TREE))
            .unwrap();
        match TokenTree::cast(list.clone()) {
            Some(tree) => token_list_hug(&meta_segments(&tree).unwrap(), 20),
            None => cfg_list_hug(&list),
        }
    }

    #[test]
    fn last_meta_hugs() {
        use Hug::{Allowed, Never, Only, Preferred};
        for (attr, expected) in [
            ("#[cfg(not(a))]", Never),
            ("#[cfg(all(a, b))]", Never),
            ("#[cfg(not(all(a, b)))]", Allowed),
            ("#[cfg(any(all(a, b)))]", Allowed),
            ("#[cfg(all(a, any(b)))]", Never),
            ("#[cfg(all(any(a), any(b)))]", Never),
            ("#[allow(dead_code)]", Never),
            ("#[allow(a, b)]", Never),
            ("#[serde(rename(serialize = \"a\"))]", Allowed),
            ("#[serde(a, rename(b))]", Never),
            ("#[a(\"b\nc\")]", Preferred),
            ("#[a(b = \"c\nd\")]", Preferred),
            ("#[a(b = r#\"c\nd\"#)]", Preferred),
            ("#[a(\nb = \"c\nd\")]", Never),
            ("#[a(\n\"b\nc\")]", Never),
            ("#[a(b = \"c\")]", Never),
            ("#[a(b = \"c\n1234567890123456789\")]", Only),
            ("#[a(b, c = \"d\ne\")]", Never),
        ] {
            assert_eq!(hug(attr), expected, "{attr}");
        }
    }
}
