//! Transformation of the Rust syntax tree into a doc, one module per syntax category

mod attr;
mod common;
mod control;
mod expr;
mod generics;
mod item;
mod pat;
mod path;
mod stmt;
mod ty;

use self::attr::format_attr;
use self::item::format_item;
use crate::comments::{count_newlines, next_code_token, trivia_from};
use crate::formatter::Edge::Word;
use crate::formatter::{Builder, Formatter, Frag};
use ra_ap_syntax::ast::{HasAttrs, HasModuleItem};
use ra_ap_syntax::{AstNode, SourceFile, SyntaxKind};

pub(crate) fn format_source_file(fmt: &Formatter, node: &SourceFile) -> Frag {
    let mut builder = Builder::new(fmt);
    let mut first = node.syntax().first_token();
    if let Some(shebang) = first.clone().filter(|t| t.kind() == SyntaxKind::SHEBANG) {
        // `#!/usr/bin/env ...` on the first line; the token includes the line break
        builder.push(fmt.text(shebang.text().trim_end(), Word));
        first = shebang.next_token();
        // The text pushed without a cursor, so the blank line after it is kept here
        let newlines = count_newlines(&shebang)
            + trivia_from(first.clone())
                .take_while(|t| t.kind() == SyntaxKind::WHITESPACE)
                .map(|t| count_newlines(&t))
                .sum::<u32>();
        let more = trivia_from(first.clone()).any(|t| t.kind() == SyntaxKind::COMMENT)
            || next_code_token(&shebang).is_some();
        if newlines > 1 && more && fmt.config().blank_lines_upper_bound > 0 {
            builder.push_blank_line();
        }
    }
    // No token precedes the comments at the start of the file, so no cursor would place them
    builder.push_comments_from(first);
    for attr in node.attrs() {
        builder.push_on_new_line(format_attr(fmt, attr));
    }
    for child in node.items() {
        builder.push_on_new_line(format_item(fmt, child));
    }
    builder.flush_trivia();
    assert!(builder.cursor().is_none());
    builder.into_frag()
}
