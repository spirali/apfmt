//! Chains: binary operators (`a && b && c`) and postfix chains (`a.b().c()?`)

use crate::formatter::Edge::Operator;
use crate::formatter::{Builder, Formatter, Frag};
use crate::layout::head::HeadLayouts;
use ra_ap_syntax::{NodeOrToken, SyntaxKind, SyntaxNode};

/// `a op b op c` on one line, or broken before every operator with indented continuation lines;
/// `rest` are the (operator, operand) pairs following `first`. A short first operand takes the
/// following operand, as in rustfmt: `a + b` / `    + c` (see `vertical_lines`).
pub(crate) fn infix_chain(fmt: &Formatter, first: Frag, rest: &[(Frag, Frag)]) -> Frag {
    chain(fmt, first, rest, true)
}

/// `a op b op c` on one line, or broken before every operator with continuation lines at the
/// indentation of the first line, e.g. or-patterns: `A` / `| B` / `| C`
pub(crate) fn unindented_infix_chain(fmt: &Formatter, first: Frag, rest: &[(Frag, Frag)]) -> Frag {
    chain(fmt, first, rest, false)
}

/// The operands of `node` separated by the operator tokens `op` (`A + B`, `A | B`), formatted with
/// `format`: the first one and the (operator, operand) pairs following it, for `infix_chain`. A
/// leading operator (`| A | B`) is dropped.
pub(crate) fn operator_chain(
    fmt: &Formatter,
    node: &SyntaxNode,
    op: SyntaxKind,
    mut format: impl FnMut(SyntaxNode) -> Frag,
) -> (Frag, Vec<(Frag, Frag)>) {
    let mut first = None;
    let mut rest = Vec::new();
    let mut operator = None;
    for child in node.children_with_tokens() {
        match child {
            NodeOrToken::Node(node) => {
                let operand = format(node);
                match operator.take() {
                    Some(operator) => rest.push((operator, operand)),
                    None => first = Some(operand),
                }
            }
            NodeOrToken::Token(token) if token.kind() == op && first.is_some() => {
                operator = Some(fmt.token(token, Operator));
            }
            NodeOrToken::Token(_) => {}
        }
    }
    (first.expect("an operand"), rest)
}

/// `a op b op c` filled on lines, as many operands on each line as fit, broken before an operator,
/// with continuation lines at the indentation of the first line, e.g. an or-pattern of short
/// alternatives: `A | B | C` / `| D | E`
pub(crate) fn unindented_infix_fill(fmt: &Formatter, first: Frag, rest: &[(Frag, Frag)]) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(first);
    for (op, operand) in rest {
        builder.push_soft_line_break();
        builder.push(Builder::concat(fmt, [op.clone(), operand.clone()]));
    }
    builder.into_frag()
}

/// `a op b op c` broken before every operator with indented continuation lines, the broken layout
/// of `infix_chain`
pub(crate) fn broken_infix_chain(fmt: &Formatter, first: Frag, rest: &[(Frag, Frag)]) -> Frag {
    vertical_lines(fmt, &first, &operator_lines(fmt, rest))
}

/// The (operator, operand) pairs joined, one per continuation line
fn operator_lines(fmt: &Formatter, rest: &[(Frag, Frag)]) -> Vec<Frag> {
    rest.iter()
        .map(|(op, operand)| Builder::concat(fmt, [op.clone(), operand.clone()]))
        .collect()
}

fn chain(fmt: &Formatter, first: Frag, rest: &[(Frag, Frag)], indent: bool) -> Frag {
    let lines = operator_lines(fmt, rest);
    let flat = Builder::concat(
        fmt,
        std::iter::once(first.clone()).chain(lines.iter().cloned()),
    );
    let vertical = if indent {
        vertical_lines(fmt, &first, &lines)
    } else {
        let mut vertical = Builder::new(fmt);
        vertical.push(first);
        vertical.push_lines(lines, false);
        vertical.into_frag()
    };
    fmt.alt_frag(fmt.flat(flat), vertical)
}

/// An element of a postfix chain: `.field`, `.method::<T>(args)` or `.await`, with the `?`
/// following it (an index is not a part of a chain, see `postfix_chain_parts` in `format/expr.rs`).
/// `head` stays on one line; `tail` (the arguments and what follows them, may be empty) may break.
pub(crate) struct ChainElement {
    pub head: Frag,
    pub tail: Frag,
}

/// The layouts of a postfix chain (see `postfix_chain_layouts`)
pub(crate) struct ChainLayouts {
    pub layouts: HeadLayouts,
    /// The root broken over more lines and a single element after its closing delimiter (`f(` /
    /// `...` / `).into()`), one of `layouts.hanging`: a chain hugged by a call may take it, the
    /// element attached to the delimiter it follows (`Err(format!(` / `...` / `).into())`)
    pub joined: Option<Frag>,
}

impl ChainLayouts {
    /// Layouts without a `joined` one
    pub fn new(layouts: HeadLayouts) -> Self {
        ChainLayouts {
            layouts,
            joined: None,
        }
    }

    /// The layouts, each transformed by `f` (e.g. with the operators around the chain)
    pub fn map(self, f: impl Fn(Frag) -> Frag) -> Self {
        ChainLayouts {
            layouts: self.layouts.map(&f),
            joined: self.joined.map(&f),
        }
    }
}

/// `root.a.b()?.c()` in the layouts of the Style Guide:
/// - on one line, if it fits (with two or more elements, within `chain_width`, as rustfmt does);
/// - broken before every `.`, each element on its own block-indented line; a short root stays with
///   the elements that follow it while its last line ends at most at the indentation of the next
///   line (`x.baz?` / `    .qux()`, also `foo(` / `...` / `).baz?`); a broken element costs a
///   split, a broken root too;
/// - all on one line except the arguments of the last element, which break (`a.b().c(` / `...` /
///   `)`), if the line fits as the single-line layout does; the Style Guide prefers the elements on
///   their own lines, so this layout costs a split, as every broken element in the previous layout
///   does.
///
/// `root` is a chain itself when it ends with an index (`a.b[i].c()`, see `postfix_chain_parts` in
/// `format/expr.rs`): broken before its own elements (`root.hanging`), the elements follow on lines at
/// the same indentation, as if it were one chain; `root.other` are its other layouts.
///
/// Returns the broken layout as `hanging` (indented continuation lines) and the others as `other`;
/// without elements (`a?`), the root. With a single element after a root that may end with a
/// delimiter, also that layout of `hanging` alone (`ChainLayouts::joined`).
pub(crate) fn postfix_chain_layouts(
    fmt: &Formatter,
    root: HeadLayouts,
    elements: &[ChainElement],
) -> ChainLayouts {
    if elements.is_empty() {
        return ChainLayouts::new(root);
    }
    let full = elements
        .iter()
        .map(|e| Builder::concat(fmt, [e.head.clone(), e.tail.clone()]))
        .collect::<Vec<_>>();
    // An element broken over more lines costs a split: the Style Guide prefers single-line elements
    let lines = full
        .iter()
        .map(|e| fmt.keep_together(e.clone()))
        .collect::<Vec<_>>();
    // A broken root chain continued by the elements, see `vertical_chain`
    let continued = root.hanging.map(|root| {
        let mut builder = Builder::new(fmt);
        builder.push(root);
        builder.push_lines(lines.iter().cloned(), true);
        builder.into_frag()
    });
    let Some(root) = root.other else {
        return ChainLayouts::new(HeadLayouts {
            hanging: continued,
            other: None,
        });
    };
    let (vertical, joined) = vertical_chain(fmt, &root, &lines);
    let vertical = fmt.alt_some(Some(vertical), continued);
    let last = elements.last().unwrap();

    // Within `chain_width` with two or more elements (as rustfmt); on one line also without a line
    // break inside a token (a multi-line string literal)
    let chain_width = (elements.len() >= 2).then_some(fmt.config().chain_width);
    let rest = &full[..full.len() - 1];
    let line = |last: &Frag| {
        let frags = std::iter::once(&root).chain(rest).chain([last]);
        Builder::concat(fmt, frags.cloned())
    };
    let width = chain_width.unwrap_or(u32::MAX);
    let mut other = fmt.flat_within(line(full.last().unwrap()), width);
    // Not built if the line up to the last arguments is too wide already
    if !last.tail.is_empty()
        && let Some(line) = fmt.flat_within(line(&last.head), width)
    {
        let mut overflow = Builder::concat(fmt, [line, fmt.multi_line(last.tail.clone())]);
        // Within `chain_width` up to the first line break, the start of the arguments included, as
        // rustfmt measures it: `a.b().c(|x| {`
        if let Some(width) = chain_width {
            overflow = fmt.first_line_within(overflow, width);
        }
        other = Some(fmt.alt_opt(fmt.dispreferred(overflow), other));
    }
    ChainLayouts {
        layouts: HeadLayouts {
            hanging: vertical,
            other,
        },
        joined,
    }
}

/// The chain broken before every element, see `postfix_chain_layouts`; `lines` are the formatted
/// elements. A short root takes the elements that follow it while it is short on a single line and
/// no comment separates them (`x.baz?` / `    .qux()`); then the lines after this head are laid out
/// by `lines_after_head`. Returns also the layout with the root broken and a single element on its
/// last line, if there is one (see `ChainLayouts::joined`).
fn vertical_chain(fmt: &Formatter, root: &Frag, lines: &[Frag]) -> (Frag, Option<Frag>) {
    // A broken root costs a split, as a broken element does (`foo(` / `    a,` / `).bar(...)`)
    let mut head = fmt.keep_together(root.clone());
    let (mut layouts, joined) = lines_after_head(fmt, &head, lines);
    for k in 0..lines.len() {
        if head.comment_follows() {
            break;
        }
        let Some(longer_head) = short_head_then(fmt, head, &lines[k]) else {
            break;
        };
        head = longer_head;
        layouts = fmt.alt_frag(lines_after_head(fmt, &head, &lines[k + 1..]).0, layouts);
    }
    (layouts, joined)
}

/// `head` followed by `line` on its last line, which ends at most at the next indentation (see
/// `Formatter::short_last_line`): `x.baz?` / `    .qux()`, `foo(` / `...` / `).await?`
fn line_after(fmt: &Formatter, head: Frag, line: &Frag) -> Frag {
    Builder::concat(fmt, [fmt.short_last_line(head), line.clone()])
}

/// `head` on a single line short enough (no line break in a token either) to end, after what
/// precedes it on its line, at most at the next indentation, followed by `line`; `None` if it is not
fn short_head_then(fmt: &Formatter, head: Frag, line: &Frag) -> Option<Frag> {
    let flat_head = fmt.flat_within(head, fmt.config().tab_spaces)?;
    Some(line_after(fmt, flat_head, line))
}

/// `head` followed by `lines`, each on its own line: block-indented after a head on a single line;
/// at the indentation of the head after a head broken over more lines, as in rustfmt (`foo(` /
/// `    a,` / `)` / `.bar()` / `.baz()`). The Style Guide's `).bar()` / `    .baz()` mixes two
/// indentations, so a head ending with a delimiter takes only a single line, the last one (`foo(` /
/// `    a,` / `).await?`).
///
/// A head that is short on a single line (within `chain_width`, a small chain by itself) is not broken
/// only to indent the lines less when one of them overflows anyway (`parse(` / `    text,` / `)` /
/// `.map_err(...)`): after it, the lines are block-indented, also when it breaks.
///
/// Returns also the layout with the head broken and a single line after its closing delimiter, if
/// there is one.
fn lines_after_head(fmt: &Formatter, head: &Frag, lines: &[Frag]) -> (Frag, Option<Frag>) {
    if lines.is_empty() {
        return (head.clone(), None);
    }
    let chain_width = fmt.config().chain_width;
    let short = fmt.min_flat_width(head).is_some_and(|w| w <= chain_width);
    let mut indented = Builder::new(fmt);
    indented.push(if short {
        head.clone()
    } else {
        fmt.flat(head.clone())
    });
    indented.push_lines(lines.iter().cloned(), true);
    let mut layout = indented.into_frag();
    let broken_head = fmt.multi_line(head.clone());
    if !short {
        let mut unindented = Builder::new(fmt);
        unindented.push(broken_head.clone());
        unindented.push_lines(lines.iter().cloned(), false);
        layout = fmt.alt_frag(layout, unindented.into_frag());
    }
    let joined = match lines {
        [line] if broken_head.may_end_with_delimiter() => Some(line_after(fmt, broken_head, line)),
        _ => None,
    };
    (fmt.alt_opt(layout, joined.clone()), joined)
}

/// `head` followed by `lines`, each on its own block-indented line (an operator chain; a postfix
/// chain is laid out by `vertical_chain`). A short head takes the lines that follow it while its
/// last line ends at most at the indentation of the next line, so it is not left alone on its line:
/// `a + b` / `    + c`, also `foo(` / `...` / `) + c`
fn vertical_lines(fmt: &Formatter, head: &Frag, lines: &[Frag]) -> Frag {
    let mut head = head.clone();
    let mut layouts: Option<Frag> = None;
    for k in 0..=lines.len() {
        let mut builder = Builder::new(fmt);
        builder.push(head.clone());
        builder.push_lines(lines[k..].iter().cloned(), true);
        let layout = builder.into_frag();
        layouts = Some(fmt.alt_opt(layout, layouts));
        if k == lines.len() {
            break;
        }
        // The head may take the next line only if its last line is short: the first fragment
        // ending with a delimiter (`foo(` / `...` / `)`), otherwise a head on a single line
        head = if k == 0 && head.may_end_with_delimiter() {
            line_after(fmt, head, &lines[k])
        } else if let Some(longer_head) = short_head_then(fmt, head, &lines[k]) {
            longer_head
        } else {
            break;
        };
    }
    layouts.unwrap()
}
