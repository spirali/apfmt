use ra_ap_syntax::SyntaxKind;
use crate::doc::{NEWLINE_DOC, NEWLINE_OR_SPACE_DOC};
use crate::formatter::{Formatter, Frag, Builder};
use crate::formatter::Edge::{Break, Nothing};

pub(crate) fn list_comma_separated(fmt: &Formatter, frags: &[Frag]) -> Frag {
    let mut builder = Builder::new(fmt);
    for (i, frag) in frags.iter().enumerate() {
        builder.push(frag.clone());
        // No trailing comma, but the source one and its comments still have to be consumed
        if i + 1 < frags.len() {
            builder.push_delimiter(SyntaxKind::COMMA);
        } else {
            builder.skip_delimiter(SyntaxKind::COMMA);
        }
    }
    builder.flush_trivia(false);
    // A line comment ends the line inside the list, so `Flat` rejects such a layout
    builder.push_line_break_if(NEWLINE_DOC, |end| end == Break);
    builder.to_frag()
}

pub(crate) fn list_nl_separated(fmt: &Formatter, frags: &[Frag]) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_newline();
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