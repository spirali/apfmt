use crate::formatter::Edge::Comma;
use crate::formatter::{Builder, Formatter, Frag};
use crate::utils::with_is_last;
use ra_ap_syntax::{AstNode, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

/// An item of a comma-separated list with the source comma following it
#[derive(Clone)]
pub(crate) struct ListItem {
    pub frag: Frag,
    /// `None` for the last item without a trailing comma
    pub comma: Option<SyntaxToken>,
}

/// The items of the comma-separated list `node` (an argument list, a parameter list, ...) with their
/// commas. `format` formats a child node, or returns `None` for children that are not items.
pub(crate) fn list_items(
    node: &SyntaxNode,
    mut format: impl FnMut(SyntaxNode) -> Option<Frag>,
) -> Vec<ListItem> {
    let mut items: Vec<ListItem> = Vec::new();
    for element in node.children_with_tokens() {
        match element {
            SyntaxElement::Node(child) => {
                if let Some(frag) = format(child) {
                    items.push(ListItem { frag, comma: None });
                }
            }
            SyntaxElement::Token(token) if token.kind() == SyntaxKind::COMMA => {
                // A comma always follows an item and is never doubled, otherwise the parser reports an error
                let last = items.last_mut().filter(|item| item.comma.is_none());
                debug_assert!(last.is_some(), "unexpected comma in {node:?}");
                if let Some(item) = last {
                    item.comma = Some(token);
                }
            }
            SyntaxElement::Token(_) => {}
        }
    }
    items
}


pub(crate) fn list_items_cast<T: AstNode>(
    node: &SyntaxNode,
    mut format: impl FnMut(T) -> Frag,
) -> Vec<ListItem> {
    list_items(node, |node| T::cast(node).map(&mut format))
}

pub(crate) fn list_comma_separated(
    fmt: &Formatter,
    items: &[ListItem],
    open: Option<SyntaxToken>,
) -> Frag {
    let mut builder = Builder::with_token(fmt, open);
    builder.flush_trivia(false);
    for (item, is_last) in with_is_last(items) {
        builder.push(item.frag.clone());
        match &item.comma {
            // The source separator, so comments in front of it stay there: `a /* c */, b`
            Some(comma) if !is_last => builder.push_token(comma.clone(), Comma),
            // No trailing comma, but the source one and its comments still have to be consumed
            _ => builder.skip_delimiter(SyntaxKind::COMMA),
        }
    }
    builder.flush_trivia(false);
    // A line comment ends the line inside the list, so `Flat` rejects such a layout
    builder.push_break_after_line_comment();
    builder.to_frag()
}

/// Items filled on lines, as many on each line as fit: `a, b, c,` / `d, e,`; with a line break after `open`,
/// indented, and a trailing comma. Comments are placed like rustfmt does: a comment before the comma or
/// ending the line belongs to the item and ends the line, a comment followed by an item on the same line
/// stays in front of it, and an own-line comment gets its own line. Comments ending the line of `open`
/// stay there, as in vertical lists.
pub(crate) fn list_fill_separated(fmt: &Formatter, items: &[ListItem], open: Option<SyntaxToken>) -> Frag {
    let mut head = Builder::with_token(fmt, open);
    head.push_comments_ending_line();
    let mut builder = Builder::with_token(fmt, head.take_cursor());
    for (item, is_last) in with_is_last(items) {
        builder.push_pre_comments();
        builder.push(item.frag.clone());
        let had_post_comment = builder.push_delimiter_with_post_comments(SyntaxKind::COMMA);
        if is_last {
            break;
        }
        if had_post_comment || builder.has_own_line_comment() {
            builder.push_newline();
        } else {
            builder.push_soft_line_break();
        }
    }
    builder.flush_trivia(false);
    builder.push_newline();
    head.push(fmt.indented_line(builder.to_frag()));
    head.to_frag()
}

/// One item per line with a trailing comma, indented; `open` is the opening delimiter and the comments
/// following it are part of the list. With `keep_blank_lines`, blank lines between items are kept
/// (up to `blank_lines_upper_bound`), e.g. between struct fields.
pub(crate) fn list_nl_separated(
    fmt: &Formatter,
    items: &[ListItem],
    open: Option<SyntaxToken>,
    keep_blank_lines: bool,
) -> Frag {
    let mut builder = Builder::with_token(fmt, open);
    builder.flush_leading_comments();
    for (item, is_last) in with_is_last(items) {
        builder.push(item.frag.clone());
        builder.push_delimiter(SyntaxKind::COMMA);
        // Blank lines in front of the closing delimiter are dropped
        if keep_blank_lines && !is_last {
            builder.ensure_newline();
        } else {
            builder.flush_trivia(false);
            builder.push_newline();
        }
    }
    fmt.nest(builder.to_frag())
}

/// `a op b op c` on one line, or broken before every operator with indented continuation lines;
/// `rest` are the (operator, operand) pairs following `first`
pub(crate) fn list_operator_separated(fmt: &Formatter, first: Frag, rest: &[(Frag, Frag)]) -> Frag {
    let mut flat = Builder::new(fmt);
    let mut vertical = Builder::new(fmt);
    flat.push(first.clone());
    vertical.push(first);
    for (op, operand) in rest {
        flat.push(op.clone());
        flat.push(operand.clone());
        let mut line = Builder::new(fmt);
        line.push(op.clone());
        line.push(operand.clone());
        vertical.push(fmt.indented_line(line.to_frag()));
    }
    fmt.alt_frag(fmt.flat(flat.to_frag()), vertical.to_frag())
}
