//! Transformation of the Rust syntax tree into a doc, one module per syntax category

mod common;
mod expr;
mod generics;
mod item;
mod pat;
mod path;
mod stmt;
mod ty;

use self::item::format_item;
use crate::formatter::Edge::Word;
use crate::formatter::{Builder, Formatter, Frag};
use ra_ap_syntax::ast::HasModuleItem;
use ra_ap_syntax::{AstNode, SourceFile, SyntaxKind};

pub(crate) fn format_source_file(fmt: &Formatter, node: &SourceFile) -> Frag {
    let mut builder = Builder::new(fmt);
    let mut first = node.syntax().first_token();
    if let Some(shebang) = first.clone().filter(|t| t.kind() == SyntaxKind::SHEBANG) {
        // `#!/usr/bin/env ...` on the first line; the token includes the line break
        builder.push(fmt.text(shebang.text().trim_end(), Word));
        first = shebang.next_token();
    }
    // No token precedes the comments at the start of the file, so no cursor would place them
    builder.push_comments_from(first);
    for child in node.items() {
        builder.push_on_new_line(format_item(fmt, child));
    }
    builder.flush_trivia(false);
    assert!(builder.cursor().is_none());
    builder.into_frag()
}
