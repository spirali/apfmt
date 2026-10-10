//! The head of a block: `if cond {`, `for pat in iter {`, `let pat = expr else {`

use crate::formatter::{Formatter, Frag};

/// The layouts of the head of a control-flow expression (`if cond`), of the initializer of a
/// `let`-`else` or of a pattern followed by more (`pat in iter`, `pat if guard =>`), by what may
/// follow them on their last line (at least one is present)
#[derive(Clone)]
pub(crate) struct HeadLayouts {
    /// The layouts ending with a continuation line: an indented one (a hanging indent, see
    /// `is_hanging_expr` in `format/expr.rs`) or the `| C` line of a broken or-pattern; `{`
    /// (`else {`) goes on its own line after them
    pub hanging: Option<Frag>,
    /// The other layouts: `{` may follow on their last line, e.g. after a chain whose last element
    /// breaks: `if a.b(|x| {` / `...` / `}) {`
    pub other: Option<Frag>,
}

impl HeadLayouts {
    pub fn all(self, fmt: &Formatter) -> Frag {
        fmt.alt_some(self.hanging, self.other)
            .expect("a head has a layout")
    }
}
