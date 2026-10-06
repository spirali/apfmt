use crate::formatter::Edge::Word;
use crate::formatter::{Formatter, Frag};
use ra_ap_syntax::ast::{Lifetime, Name};

pub(super) fn format_name(fmt: &Formatter, name: Name) -> Frag {
    fmt.token(name.ident_token().unwrap(), Word)
}

/// `'a`
pub(super) fn format_lifetime(fmt: &Formatter, lifetime: Lifetime) -> Frag {
    fmt.token(lifetime.lifetime_ident_token().unwrap(), Word)
}
