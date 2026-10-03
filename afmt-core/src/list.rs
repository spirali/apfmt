use crate::formatter::{Builder, Formatter, Frag};
use ra_ap_syntax::{SyntaxKind, SyntaxToken};

/// `open` is the opening delimiter; the comments following it are part of the list
pub(crate) fn list_comma_separated(
    fmt: &Formatter,
    frags: &[Frag],
    open: Option<SyntaxToken>,
) -> Frag {
    let mut builder = Builder::with_token(fmt, open);
    builder.flush_trivia(false);
    for (i, frag) in frags.iter().enumerate() {
        builder.push(frag.clone());
        // No trailing comma, but the source one and its comments still have to be consumed
        let emit = i + 1 < frags.len();
        builder.consume_delimiter(SyntaxKind::COMMA, emit);
    }
    builder.flush_trivia(false);
    // A line comment ends the line inside the list, so `Flat` rejects such a layout
    builder.push_break_after_line_comment();
    builder.to_frag()
}

/// `open` is the opening delimiter; the comments following it are part of the list
/// Items filled on lines, as many on each line as fit: `a, b, c,` / `d, e,`; with a line break after `open`,
/// indented, and a trailing comma. Comments are placed like rustfmt does: a comment before the comma or
/// ending the line belongs to the item and ends the line, a comment followed by an item on the same line
/// stays in front of it, and an own-line comment gets its own line. Comments ending the line of `open`
/// stay there, as in vertical lists.
pub(crate) fn list_fill_separated(fmt: &Formatter, frags: &[Frag], open: Option<SyntaxToken>) -> Frag {
    let mut head = Builder::with_token(fmt, open);
    head.push_comments_ending_line();
    let mut builder = Builder::with_token(fmt, head.take_cursor());
    let mut had_post_comment = false;
    for (i, frag) in frags.iter().enumerate() {
        if i > 0 {
            if had_post_comment || builder.has_own_line_comment() {
                builder.push_newline();
            } else {
                builder.push_soft_line_break();
            }
        }
        builder.push_pre_comments();
        builder.push(frag.clone());
        had_post_comment = builder.push_delimiter_with_post_comments(SyntaxKind::COMMA);
    }
    builder.flush_trivia(false);
    builder.push_newline();
    head.push(fmt.indented_line(builder.to_frag()));
    head.to_frag()
}

pub(crate) fn list_nl_separated(
    fmt: &Formatter,
    frags: &[Frag],
    open: Option<SyntaxToken>,
) -> Frag {
    let mut builder = Builder::with_token(fmt, open);
    builder.flush_leading_comments();
    for frag in frags.iter() {
        builder.push(frag.clone());
        builder.push_delimiter(SyntaxKind::COMMA);
        builder.flush_trivia(false);
        builder.push_newline();
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
