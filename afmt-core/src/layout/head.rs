//! The head of a block: `if cond {`, `for pat in iter {`, `let pat = expr else {`

use crate::formatter::{Formatter, Frag};

/// The layouts of the head of a control-flow expression (`if cond`), of the initializer of a
/// `let`-`else` or of a pattern followed by more (`pat in iter`, `pat if guard =>`), by what may
/// follow them on their last line (at least one is present)
#[derive(Clone)]
pub(crate) struct HeadLayouts {
    /// The layouts ending with a continuation line: an indented one (a hanging indent: a broken
    /// chain, also behind operators, or `a && b`, see `operator_chain_parts` and `is_hanging_expr` in
    /// `format/expr.rs`) or the `| C` line of a broken or-pattern; `{`
    /// (`else {`) goes on its own line after them
    pub hanging: Option<Frag>,
    /// The other layouts: `{` may follow on their last line, e.g. after a chain whose last element
    /// breaks: `if a.b(|x| {` / `...` / `}) {`
    pub other: Option<Frag>,
}

impl HeadLayouts {
    /// A head with only `other` layouts
    pub fn other(frag: Frag) -> Self {
        HeadLayouts {
            hanging: None,
            other: Some(frag),
        }
    }

    /// The layouts, each transformed by `f` (e.g. followed by more)
    pub fn map(self, f: impl Fn(Frag) -> Frag) -> Self {
        HeadLayouts {
            hanging: self.hanging.map(&f),
            other: self.other.map(&f),
        }
    }

    /// All layouts as hanging ones, so `{` goes on its own line after them: the head is on an
    /// indented line, e.g. after comments ending the line in front of it
    pub fn into_hanging(self, fmt: &Formatter) -> Self {
        HeadLayouts {
            hanging: Some(self.all(fmt)),
            other: None,
        }
    }

    pub fn all(self, fmt: &Formatter) -> Frag {
        fmt.alt_some(self.hanging, self.other)
            .expect("a head has a layout")
    }
}

/// A pattern followed by the rest of a head, joined by `join` (`let pat = expr`, `pat in iter`),
/// `all_pat` being all layouts of `pat`. After a pattern on a single line, the rest as it is; after a
/// broken one, `{` follows only a rest spanning more lines, whose last line is then closing
/// delimiters (Style Guide), and after a rest on a single line (`} = x`, `} in x`) the head hangs, so
/// `{` goes on its own line. A broken or-pattern ends on a `| C` line, so it hangs too.
pub(crate) fn pattern_head(
    fmt: &Formatter,
    pat: &HeadLayouts,
    all_pat: &Frag,
    rest: &HeadLayouts,
    join: impl Fn(Frag, Frag) -> Frag,
) -> HeadLayouts {
    let other = (rest.other.clone())
        .zip(pat.other.clone())
        .map(|(rest, pat)| {
            fmt.alt_frag(
                join(fmt.flat(pat.clone()), rest.clone()),
                join(fmt.multi_line(pat), fmt.multi_line(rest)),
            )
        });
    let mut hanging = (rest.hanging.clone()).map(|rest| join(all_pat.clone(), rest));
    if let (Some(pat), Some(rest)) = (&pat.hanging, &rest.other) {
        hanging = fmt.alt_some(hanging, Some(join(pat.clone(), rest.clone())));
    }
    if let (Some(pat), Some(rest)) = (&pat.other, &rest.other) {
        let broken_pat = join(fmt.multi_line(pat.clone()), fmt.flat(rest.clone()));
        hanging = fmt.alt_some(hanging, Some(broken_pat));
    }
    HeadLayouts { hanging, other }
}
