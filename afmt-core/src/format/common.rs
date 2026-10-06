use crate::formatter::Edge::{BraceClose, BraceOpen, Close, Open, Word};
use crate::formatter::{Builder, Formatter, Frag};
use ra_ap_syntax::SyntaxToken;
use ra_ap_syntax::ast::{Lifetime, Name};

pub(super) fn format_name(fmt: &Formatter, name: Name) -> Frag {
    fmt.token(name.ident_token().unwrap(), Word)
}

/// `'a`
pub(super) fn format_lifetime(fmt: &Formatter, lifetime: Lifetime) -> Frag {
    fmt.token(lifetime.lifetime_ident_token().unwrap(), Word)
}

/// `(inner)`, `[inner]` or `<inner>`
pub(super) fn format_parenthesized(
    fmt: &Formatter,
    open: SyntaxToken,
    inner: Frag,
    close: SyntaxToken,
) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_token(open, Open);
    builder.push(inner);
    builder.push_token(close, Close);
    builder.into_frag()
}

/// `{ inner }` of a block or a struct literal
pub(super) fn format_braced(
    fmt: &Formatter,
    open: SyntaxToken,
    inner: Frag,
    close: SyntaxToken,
) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_token(open, BraceOpen);
    builder.push(inner);
    builder.push_token(close, BraceClose);
    builder.into_frag()
}
