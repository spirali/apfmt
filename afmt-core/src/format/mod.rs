//! Transformation of the Rust syntax tree into a doc, one module per syntax category

mod common;
mod expr;
mod item;
mod pat;
mod path;
mod stmt;
mod ty;

use self::item::format_item;
use crate::formatter::{Builder, Formatter, Frag};
use ra_ap_syntax::ast::HasModuleItem;
use ra_ap_syntax::{SourceFile, SyntaxToken};

pub(crate) fn format_source_file(fmt: &Formatter, node: &SourceFile) -> Frag {
    let mut builder = Builder::new(fmt);
    for child in node.items() {
        builder.push_on_new_line(format_item(fmt, child));
    }
    builder.flush_trivia(false);
    assert!(builder.cursor().is_none());
    builder.to_frag()
}
