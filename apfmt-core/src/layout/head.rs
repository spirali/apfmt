//! The head of a block: `if cond {`, `for pat in iter {`, `let pat = expr else {`

use crate::formatter::{Formatter, Frag};

/// The layouts of a head (`if cond`, a `let`-`else` initializer, `pat in iter`), by what may follow
/// them on their last line (at least one is present)
#[derive(Clone)]
pub(crate) struct HeadLayouts {
    /// Layouts ending with a continuation line (a broken chain, `a && b`, a `| C` line); `{` goes
    /// on its own line after them
    pub hanging: Option<Frag>,
    /// `{` may follow on their last line: `if a.b(|x| {` / `...` / `}) {`
    pub other: Option<Frag>,
}

impl HeadLayouts {
    pub fn other(frag: Frag) -> Self {
        HeadLayouts {
            hanging: None,
            other: Some(frag),
        }
    }

    /// A head that hangs when broken (`a && b`, `A | B`): `{` follows only its single line, so a
    /// broken head comes only when it does not fit (`if a && b` / `{`)
    pub fn hanging_when_broken(fmt: &Formatter, frag: Frag) -> Self {
        HeadLayouts {
            hanging: Some(fmt.multi_line(frag.clone())),
            other: Some(fmt.flat(frag)),
        }
    }

    /// What may follow only the single line of `other`, e.g. `else {` after `f(` / `    a,` / `)?`
    pub fn other_hanging_when_broken(self, fmt: &Formatter) -> Self {
        let Some(other) = self.other else {
            return self;
        };
        HeadLayouts {
            hanging: fmt.alt_some(self.hanging, Some(fmt.multi_line(other.clone()))),
            other: Some(fmt.flat(other)),
        }
    }

    pub fn map(self, f: impl Fn(Frag) -> Frag) -> Self {
        HeadLayouts {
            hanging: self.hanging.map(&f),
            other: self.other.map(&f),
        }
    }

    /// All layouts as hanging ones, e.g. a head on an indented line after comments
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

/// `pat`, `join` and the rest of a head (`let pat = expr`). After a broken pattern, `{` follows
/// only a rest spanning more lines (Style Guide); otherwise the head hangs.
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
