use ra_ap_syntax::SyntaxToken;
use crate::doc::{DocId, DocNode, NEWLINE_DOC};
use crate::formatter::Formatter;

/// Puts a single space after the opening and before the closing sigil of a single-line
/// block comment (`/*a*/` -> `/* a */`). Returns `None` when the text stays as it is.
/// Doc comments and empty comments are left alone.
pub(crate) fn normalize_block_comment(text: &str) -> Option<String> {
    let inner = text.strip_prefix("/*")?.strip_suffix("*/")?;
    if inner.contains('\n') || inner.starts_with('*') || inner.starts_with('!') {
        return None;
    }
    let trimmed = inner.trim();
    if trimmed.is_empty() {
        return None;
    }
    let normalized = format!("/* {trimmed} */");
    (normalized != text).then_some(normalized)
}

fn source_column(token: &SyntaxToken) -> usize {
    let mut col = 0;
    let mut element = token.prev_token();
    while let Some(token) = element {
        let text = token.text();
        if let Some(pos) = text.rfind('\n') {
            return col + unicode_width::UnicodeWidthStr::width(&text[pos + 1..]);
        }
        col += unicode_width::UnicodeWidthStr::width(text);
        element = token.prev_token();
    }
    col
}

pub(crate) fn multiline_comment_doc(fmt: &Formatter, token: &SyntaxToken) -> Option<DocId> {
    let (first, rest) = token.text().split_once('\n')?;
    let col = source_column(token);
    let is_blank = |line: &str| line.trim().is_empty();
    if !rest.split('\n').all(|line| {
        is_blank(line) || (line.len() >= col && line.as_bytes()[..col].iter().all(|c| *c == b' '))
    }) {
        return None;
    }
    let mut doc_id = fmt.make_node(DocNode::Text(first.to_string()));
    for line in rest.split('\n') {
        doc_id = fmt.concat(doc_id, NEWLINE_DOC);
        if !is_blank(line) {
            doc_id = fmt.concat(doc_id, fmt.make_node(DocNode::Text(line[col..].to_string())));
        }
    }
    Some(fmt.make_node(DocNode::Align(doc_id)))
}
