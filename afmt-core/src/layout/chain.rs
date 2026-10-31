//! Chains: binary operators (`a && b && c`) and postfix chains (`a.b().c()?`)

use crate::doc::WeakPreference;
use crate::formatter::Edge::Operator;
use crate::formatter::{Builder, Formatter, Frag};
use crate::layout::head::HeadLayouts;
use ra_ap_syntax::{NodeOrToken, SyntaxKind, SyntaxNode};

/// `a op b op c` on one line, or broken before every operator with indented continuation lines. A
/// short first operand takes the next one, as in rustfmt: `a + b` / `    + c`.
pub(crate) fn infix_chain(fmt: &Formatter, first: Frag, rest: &[(Frag, Frag)]) -> Frag {
    chain(fmt, first, rest, true)
}

/// As `infix_chain`, but continuation lines at the indentation of the first line: `A` / `| B` / `|
/// C`
pub(crate) fn unindented_infix_chain(fmt: &Formatter, first: Frag, rest: &[(Frag, Frag)]) -> Frag {
    chain(fmt, first, rest, false)
}

/// The operands of `node` separated by the operator tokens `op`, for `infix_chain`; a leading
/// operator (`| A | B`) is dropped
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

/// `a op b op c` filled on lines, broken before an operator, unindented: `A | B | C` / `| D | E`
pub(crate) fn unindented_infix_fill(fmt: &Formatter, first: Frag, rest: &[(Frag, Frag)]) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(first);
    for (op, operand) in rest {
        builder.push_soft_line_break();
        builder.push(Builder::concat(fmt, [op.clone(), operand.clone()]));
    }
    builder.into_frag()
}

/// `first op operand` with `first` on a single line within `max_width` and `operand` broken on its
/// line: `x = 0 + f(` / `    a,` / `);`
pub(crate) fn short_first_operand(
    fmt: &Formatter,
    first: &Frag,
    op: &Frag,
    operand: &Frag,
    max_width: u32,
) -> Option<Frag> {
    if first.comment_follows() || op.comment_follows() {
        return None;
    }
    let first = fmt.flat_within(first.clone(), max_width)?;
    Some(Builder::concat(fmt, [first, op.clone(), fmt.multi_line(operand.clone())]))
}

/// The operands but the last on a single line, the last one broken on it after its operator, as
/// rustfmt does with a block: `a && b && {` / `    ...` / `}`
pub(crate) fn flat_operands_then_last(
    fmt: &Formatter,
    first: &Frag,
    rest: &[(Frag, Frag)],
) -> Option<Frag> {
    let ((op, last), front) = rest.split_last()?;
    let front = Builder::concat(
        fmt,
        std::iter::once(first.clone()).chain(operator_lines(fmt, front)),
    );
    if front.comment_follows() || op.comment_follows() {
        return None;
    }
    let front = fmt.flat_if_possible(front)?;
    Some(Builder::concat(fmt, [front, op.clone(), fmt.multi_line(last.clone())]))
}

/// The broken layout of `infix_chain`
pub(crate) fn broken_infix_chain(fmt: &Formatter, first: Frag, rest: &[(Frag, Frag)]) -> Frag {
    vertical_lines(fmt, &first, &operator_lines(fmt, rest))
}

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

/// `.field`, `.method::<T>(args)` or `.await`, with its `?`. `head` stays on one line; `tail` (the
/// arguments and what follows, may be empty) may break.
pub(crate) struct ChainElement {
    pub head: Frag,
    pub tail: Frag,
}

pub(crate) struct ChainLayouts {
    pub layouts: HeadLayouts,
    /// The root broken and a single element after its closing delimiter (`f(` / `...` /
    /// `).into()`), one of `layouts.hanging`, for a chain hugged by a call
    pub joined: Option<Frag>,
}

impl ChainLayouts {
    pub fn new(layouts: HeadLayouts) -> Self {
        ChainLayouts {
            layouts,
            joined: None,
        }
    }

    pub fn map(self, f: impl Fn(Frag) -> Frag) -> Self {
        ChainLayouts {
            layouts: self.layouts.map(&f),
            joined: self.joined.map(&f),
        }
    }
}

/// `root.a.b()?.c()` in the layouts of the Style Guide: on one line (within `chain_width`), broken
/// before every `.` (a short root keeps the elements it can, `x.baz?` / `    .qux()`), or on one
/// line except the last arguments (costs a split). A `root` ending with an index is a chain itself,
/// its elements continued at the same indentation. `suffix` (indexes after the chain) follows the
/// last element and does not count in `chain_width`.
pub(crate) fn postfix_chain_layouts(
    fmt: &Formatter,
    root: HeadLayouts,
    elements: &[ChainElement],
    suffix: &Frag,
) -> ChainLayouts {
    let then_suffix = |frag: Frag| Builder::concat(fmt, [frag, suffix.clone()]);
    if elements.is_empty() {
        return ChainLayouts::new(root.map(then_suffix));
    }
    let full = elements
        .iter()
        .map(|e| Builder::concat(fmt, [e.head.clone(), e.tail.clone()]))
        .collect::<Vec<_>>();
    let mut lines = full
        .iter()
        .map(|e| fmt.keep_together(e.clone()))
        .collect::<Vec<_>>();
    let last_line = lines.pop().unwrap();
    lines.push(then_suffix(last_line));
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

    // Within `chain_width` with two or more elements (as rustfmt), without a multi-line token
    let chain_width = (elements.len() >= 2).then_some(fmt.config().chain_width);
    let rest = &full[..full.len() - 1];
    let line = |last: &Frag| {
        let frags = std::iter::once(&root).chain(rest).chain([last]);
        Builder::concat(fmt, frags.cloned())
    };
    let width = chain_width.unwrap_or(u32::MAX);
    let mut other = fmt.flat_within(line(full.last().unwrap()), width);
    if !last.tail.is_empty()
        && let Some(line) = fmt.flat_within(line(&last.head), width)
    {
        let mut overflow = Builder::concat(fmt, [line, fmt.multi_line(last.tail.clone())]);
        // Within `chain_width` up to the first line break, as rustfmt measures it: `a.b().c(|x| {`
        if let Some(width) = chain_width {
            overflow = fmt.first_line_within(overflow, width);
        }
        other = Some(fmt.alt_opt(fmt.dispreferred(overflow), other));
    }
    let other = other.map(then_suffix);
    ChainLayouts {
        layouts: HeadLayouts {
            hanging: vertical,
            other,
        },
        joined,
    }
}

/// The chain broken before every element. A short root takes the elements following it while it is
/// short (`x.baz?` / `    .qux()`). Also returns the `ChainLayouts::joined` layout.
fn vertical_chain(fmt: &Formatter, root: &Frag, lines: &[Frag]) -> (Frag, Option<Frag>) {
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

/// `head` followed by `line` on its short last line: `x.baz?` / `    .qux()`, `foo(` / `...` /
/// `).await?`
fn line_after(fmt: &Formatter, head: Frag, line: &Frag) -> Frag {
    Builder::concat(fmt, [fmt.short_last_line(head), line.clone()])
}

/// `head` on a single line short enough to end at most at the next indentation, followed by `line`
fn short_head_then(fmt: &Formatter, head: Frag, line: &Frag) -> Option<Frag> {
    let flat_head = fmt.flat_within(head, fmt.config().tab_spaces)?;
    Some(line_after(fmt, flat_head, line))
}

/// `head` followed by `lines`: block-indented after a flat head, at its indentation after a broken
/// one (`foo(` / `    a,` / `)` / `.bar()`, as rustfmt), or a single line after `)` (`).await?`).
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
        // At the indentation of the head only after a closing delimiter; after a continuation line,
        // they would be left of it (`(a` / `    + b)` / `    .c()`)
        let mut unindented = Builder::new(fmt);
        unindented.push(fmt.short_last_line(broken_head.clone()));
        unindented.push_lines(lines.iter().cloned(), false);
        let unindented = fmt.weak_preference(unindented.into_frag(), WeakPreference::PREFERRED);
        let mut indented = Builder::new(fmt);
        indented.push(broken_head.clone());
        indented.push_lines(lines.iter().cloned(), true);
        layout = fmt.alt_frag(fmt.alt_frag(layout, indented.into_frag()), unindented);
    }
    let joined = match lines {
        [line] if broken_head.may_end_with_delimiter() => Some(line_after(fmt, broken_head, line)),
        _ => None,
    };
    (fmt.alt_opt(layout, joined.clone()), joined)
}

/// `head` followed by `lines`, each block-indented (an operator chain). A short head takes the next
/// lines while its last line is short: `a + b` / `    + c`, also `foo(` / `...` / `) + c`
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
        // The head may take the next line only after a closing delimiter, broken only where it does
        // not fit on one line, whatever the lines after it gain; otherwise a head on a single line
        head = if k == 0 && head.may_end_with_delimiter() {
            line_after(fmt, fmt.flat_where_fits(head, 0), &lines[k])
        } else if let Some(longer_head) = short_head_then(fmt, head, &lines[k]) {
            longer_head
        } else {
            break;
        };
    }
    layouts.unwrap()
}
