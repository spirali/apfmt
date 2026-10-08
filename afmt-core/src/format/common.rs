use crate::formatter::Edge::{Comma, Word};
use crate::formatter::{Builder, Formatter, Frag};
use crate::layout::list::Delimiters;
use ra_ap_syntax::ast::{Label, Lifetime, Name};
use ra_ap_syntax::{SyntaxKind, SyntaxNode, SyntaxToken};

pub(super) fn format_name(fmt: &Formatter, name: Name) -> Frag {
    fmt.token(name.ident_token().unwrap(), Word)
}

/// `'a`
pub(super) fn format_lifetime(fmt: &Formatter, lifetime: Lifetime) -> Frag {
    fmt.token(lifetime.lifetime_ident_token().unwrap(), Word)
}

/// Pushes the modifier keywords among the tokens of `node` in the source order: `unsafe`, `async`,
/// `move`, `const`, `gen` and `static` of a block (`async move {`) or a closure (`move |x|`)
pub(super) fn push_modifiers(builder: &mut Builder, node: &SyntaxNode) {
    for token in node.children_with_tokens().filter_map(|c| c.into_token()) {
        match token.kind() {
            SyntaxKind::UNSAFE_KW
            | SyntaxKind::ASYNC_KW
            | SyntaxKind::MOVE_KW
            | SyntaxKind::CONST_KW
            | SyntaxKind::GEN_KW
            | SyntaxKind::STATIC_KW => builder.push_keyword(token),
            _ => {}
        }
    }
}

/// `[value; len]` of the array expression or type `node`
pub(super) fn format_repeat(
    fmt: &Formatter,
    node: &SyntaxNode,
    value: Frag,
    semicolon: SyntaxToken,
    len: Frag,
) -> Frag {
    let mut inner = Builder::new(fmt);
    inner.push(value);
    inner.push_token(semicolon, Comma);
    inner.push(len);
    Delimiters::brackets_of(node).enclose(fmt, inner.into_frag())
}

/// Pushes the label of a block or a loop: `'a: `
pub(super) fn push_label(fmt: &Formatter, builder: &mut Builder, label: Option<Label>) {
    if let Some(label) = label {
        builder.push(format_lifetime(fmt, label.lifetime().unwrap()));
        builder.push_token(label.colon_token().unwrap(), Comma);
    }
}
