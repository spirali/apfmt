use ra_ap_syntax::{SyntaxKind, SyntaxToken};
use crate::doc::{NEWLINE_DOC, NEWLINE_OR_SPACE_DOC};
use crate::formatter::{Formatter, Frag, Builder};
use crate::formatter::Edge::{Break, Nothing};

/// `open` is the opening delimiter; the comments following it are part of the list
pub(crate) fn list_comma_separated(fmt: &Formatter, frags: &[Frag], open: Option<SyntaxToken>) -> Frag {
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
    builder.push_line_break_if(NEWLINE_DOC, |end| end == Break);
    builder.to_frag()
}

/// `open` is the opening delimiter; the comments following it are part of the list
pub(crate) fn list_nl_separated(fmt: &Formatter, frags: &[Frag], open: Option<SyntaxToken>) -> Frag {
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

pub(crate) fn list_comma_mix_separated(fmt: &Formatter, frags: &[Frag]) -> Frag {
    let mut builder = Builder::new(fmt);
    let mut it = frags.iter();
    let Some(frag) = it.next() else {
        return Frag::empty();
    };
    builder.push(frag.clone());
    for d in it {
        builder.push(fmt.text(",".to_string(), Nothing));
        builder.push(Frag::new_simple(NEWLINE_OR_SPACE_DOC, Nothing, None));
        builder.push(d.clone());
    }
    builder.to_frag()
}