//! Chains: binary operators (`a && b && c`) and postfix chains (`a.b().c()?`)

use crate::formatter::Edge::Operator;
use crate::formatter::{Builder, Formatter, Frag};
use crate::layout::list::FlatLimit;
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

/// An element of a postfix chain: `.field`, `.method::<T>(args)` or `.await`, with the `?` and
/// indexes following it. `head` stays on one line; `tail` (the arguments and what follows them, may
/// be empty) may break.
pub(crate) struct ChainElement {
    pub head: Frag,
    pub tail: Frag,
}

/// `root.a.b()?.c()` in the layouts of the Style Guide:
/// - on one line, if it fits (with two or more elements, within `chain_width`, as rustfmt does);
/// - broken before every `.`, each element on its own block-indented line; a short root stays with
///   the elements that follow it while its last line ends at most at the indentation of the next
///   line (`x.baz?` / `    .qux()`, also `foo(` / `...` / `).baz?`);
/// - all on one line except the arguments of the last element, which break (`a.b().c(` / `...` /
///   `)`), if the line fits as the single-line layout does; the Style Guide prefers the elements on
///   their own lines, so this layout costs a split, as every broken element in the previous layout
///   does.
///
/// Returns the broken layout, with indented continuation lines (a hanging indent), and the others;
/// without elements (`a?[0]`), there is only the root.
pub(crate) fn postfix_chain_layouts(
    fmt: &Formatter,
    root: Frag,
    elements: &[ChainElement],
) -> (Option<Frag>, Option<Frag>) {
    let Some(last) = elements.last() else {
        return (None, Some(root));
    };
    let full = elements
        .iter()
        .map(|e| Builder::concat(fmt, [e.head.clone(), e.tail.clone()]))
        .collect::<Vec<_>>();
    let vertical = vertical_chain(fmt, &root, &full);

    let limit = if elements.len() >= 2 {
        FlatLimit::Within(fmt.config().chain_width)
    } else {
        FlatLimit::MaxWidth
    };
    // The single-line width of the root and the elements in front of the last one, measured once; the
    // layouts are checked exactly by `FlatLimit::apply` (comments in between count too)
    let rest = &full[..full.len() - 1];
    let rest_width = std::iter::once(&root)
        .chain(rest)
        .try_fold(0, |w, f| Some(w + fmt.flat_width(f)?));
    let fits = |last: &Frag| {
        let width = rest_width.zip(fmt.flat_width(last)).map(|(a, b)| a + b);
        width.is_some_and(|w| match limit {
            FlatLimit::Within(limit) => w <= limit as usize,
            FlatLimit::MaxWidth => true,
        })
    };
    let line = |last: &Frag| {
        let frags = std::iter::once(&root).chain(rest).chain([last]);
        limit.apply(fmt, Builder::concat(fmt, frags.cloned()))
    };

    let mut other = None;
    let full_last = full.last().unwrap();
    if fits(full_last) {
        other = line(full_last);
    }
    if !last.tail.is_empty()
        && fits(&last.head)
        && let Some(line) = line(&last.head)
    {
        let overflow = Builder::concat(fmt, [line, fmt.multi_line(last.tail.clone())]);
        other = Some(fmt.alt_opt(fmt.dispreferred(overflow), other));
    }
    (Some(vertical), other)
}

/// The chain broken before every element, see `postfix_chain_layouts`; `elements` are formatted
fn vertical_chain(fmt: &Formatter, root: &Frag, elements: &[Frag]) -> Frag {
    // An element broken over more lines costs a split: the Style Guide prefers single-line elements
    let lines = elements
        .iter()
        .map(|e| fmt.keep_together(e.clone()))
        .collect::<Vec<_>>();
    vertical_lines(fmt, root, &lines)
}

/// `head` followed by `lines`, each on its own block-indented line. A short head takes the lines
/// that follow it while its last line ends at most at the indentation of the next line, so it is
/// not left alone on its line: `x.baz?` / `    .qux()`, `a + b` / `    + c`, also `foo(` / `...` /
/// `).baz?`
fn vertical_lines(fmt: &Formatter, head: &Frag, lines: &[Frag]) -> Frag {
    let indent_width = fmt.config().indent_width as usize;
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
        // The head may take the next line only if its last line can be short: it is short on a
        // single line, or it is the first fragment ending with a delimiter (`foo(` / `...` / `)`)
        let short = (k == 0 && head.may_end_with_delimiter())
            || fmt.flat_width(&head).is_some_and(|w| w <= indent_width);
        if !short {
            break;
        }
        head = Builder::concat(fmt, [fmt.short_last_line(head), lines[k].clone()]);
    }
    layouts.unwrap()
}
