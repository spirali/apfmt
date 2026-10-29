//! Transformation of the Rust syntax tree into a doc, one module per syntax category

mod attr;
mod chain;
mod closure;
mod common;
mod control;
mod expr;
mod expr_list;
mod generics;
mod item;
mod macros;
mod pat;
mod path;
mod stmt;
mod ty;

use self::attr::format_attr;
use self::item::format_item;
use crate::comments::{
    count_newlines, first_code_token, is_code_kind, is_comment_kind, next_code_token, trivia_from,
};
use crate::formatter::Edge::Word;
use crate::formatter::{Builder, Formatter, Frag};
use ra_ap_syntax::ast::{HasAttrs, HasModuleItem};
use ra_ap_syntax::{AstNode, NodeOrToken, SourceFile, SyntaxKind, SyntaxNode, SyntaxToken};

/// Given to `emit` in chunks of finished lines: a chunk ends before a top-level item, so `emit` can
/// render it and clear the arena
pub(crate) fn format_source_file(fmt: &Formatter, node: &SourceFile, mut emit: impl FnMut(Frag)) {
    let mut builder = Builder::new(fmt);
    let mut first = node.syntax().first_token();
    if let Some(shebang) = first.clone().filter(|t| t.kind() == SyntaxKind::SHEBANG) {
        // The token includes the line break
        builder.push(fmt.text(shebang.text().trim_end(), Word));
        first = shebang.next_token();
        // Pushed without a cursor, so the blank line after it is kept here
        let newlines = count_newlines(&shebang)
            + trivia_from(first.clone())
                .take_while(|t| t.kind() == SyntaxKind::WHITESPACE)
                .map(|t| count_newlines(&t))
                .sum::<u32>();
        let more = trivia_from(first.clone()).any(|t| is_comment_kind(t.kind()))
            || next_code_token(&shebang).is_some();
        if newlines > 1 && more && fmt.config().blank_lines_upper_bound > 0 {
            builder.push_blank_line();
        }
    }
    // No token precedes the comments at the start of the file
    builder.push_comments_from(first);
    // The chunk before is emitted before `format` runs, whose docs must survive the clearing
    let mut push_line = |builder: &mut Builder, format: &dyn Fn() -> Frag| {
        if !builder.is_empty() {
            builder.flush_trivia_and_end_line();
        }
        if let Some(lines) = builder.split_off_finished_lines() {
            emit(lines);
        }
        builder.push(format());
    };
    for attr in node.attrs() {
        push_line(&mut builder, &|| format_attr(fmt, attr.clone()));
    }
    for child in node.items() {
        push_line(&mut builder, &|| format_item(fmt, child.clone()));
    }
    builder.flush_trivia();
    assert!(builder.cursor().is_none());
    emit(builder.into_frag());
}

/// The caller places the leading comments and attributes
fn first_token_after_attrs(node: &SyntaxNode) -> SyntaxToken {
    node.children_with_tokens()
        .find(|e| is_code_kind(e.kind()) && e.kind() != SyntaxKind::ATTR)
        .and_then(|e| match e {
            NodeOrToken::Token(token) => Some(token),
            NodeOrToken::Node(node) => node.first_token(),
        })
        .unwrap()
}

/// Copied from the source, reindented, without the leading comments and attributes
pub(super) fn format_reindented(fmt: &Formatter, node: &SyntaxNode) -> Frag {
    let (first, last) = (first_token_after_attrs(node), node.last_token().unwrap());
    fmt.reindented_range(&first, &last, Word, Word)
}

/// `format_reindented`, recorded as not supported because of `what`
pub(super) fn format_unsupported(
    fmt: &Formatter,
    node: &SyntaxNode,
    what: impl Into<String>,
) -> Frag {
    let first = first_token_after_attrs(node);
    fmt.unsupported(&first, &node.last_token().unwrap(), what.into())
}

/// With the attributes of `node`, which are lost otherwise
pub(super) fn format_unsupported_with_attrs(
    fmt: &Formatter,
    node: &SyntaxNode,
    what: impl Into<String>,
) -> Frag {
    fmt.unsupported(
        &first_code_token(node),
        &node.last_token().unwrap(),
        what.into(),
    )
}

/// E.g. `MACRO_EXPR`
pub(super) fn format_unsupported_kind(fmt: &Formatter, node: &SyntaxNode) -> Frag {
    format_unsupported(fmt, node, format!("{:?}", node.kind()))
}
