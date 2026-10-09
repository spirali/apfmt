//! Building docs: the `Formatter` (doc arena and combinators), fragments (`Frag`) and the `Builder`
//! that glues fragments together and places the source trivia (comments, blank lines)

mod builder;
mod frag;
mod trivia;

use crate::comments::{multiline_comment_lines, normalize_block_comment};
use crate::config::Config;
use crate::doc::DocNode::Newline;
use crate::doc::{Cost, DocArena, DocId, DocNode, EMPTY_DOC, NEWLINE_DOC, NEWLINE_OR_SPACE_DOC};
use crate::formatter::Edge::{Keyword, LineStart, Nothing, Word};
use crate::resolver::Resolver;
pub(crate) use builder::Builder;
use frag::{Branch, Branches};
pub(crate) use frag::{Edge, Frag};
use ra_ap_syntax::{SyntaxNode, SyntaxToken, TextRange};
use std::cell::RefCell;

/// Source text that afmt cannot format yet, copied unchanged (see `Formatter::unsupported`)
pub(crate) struct Unsupported {
    /// The copied source text
    pub range: TextRange,
    /// What is not supported, e.g. `MACRO_EXPR`
    pub what: String,
}

pub(crate) struct Formatter<'a> {
    doc_arena: RefCell<DocArena>,
    config: &'a Config,
    unsupported: RefCell<Vec<Unsupported>>,
}

impl<'a> Formatter<'a> {
    pub fn new(config: &'a Config) -> Self {
        Formatter {
            config,
            doc_arena: RefCell::new(DocArena::new()),
            unsupported: RefCell::new(Vec::new()),
        }
    }

    #[inline]
    pub fn config(&self) -> &'a Config {
        self.config
    }

    fn make_node(&self, doc_node: DocNode) -> DocId {
        self.doc_arena.borrow_mut().make(doc_node)
    }

    pub fn token(&self, token: SyntaxToken, edge: Edge) -> Frag {
        Frag::new_simple(
            self.make_node(DocNode::Token(token.clone())),
            edge,
            Some(token),
        )
    }

    /// A token with different edges on its sides, e.g. the negative `!` of `impl !Send`: a space in
    /// front of it (`Word`), none after it (`Tight`)
    pub fn token_with_edges(&self, token: SyntaxToken, start: Edge, end: Edge) -> Frag {
        Frag::new(
            self.make_node(DocNode::Token(token.clone())),
            start,
            end,
            Some(token),
        )
    }

    /// Text that is not in the source (e.g. an inserted `"C"`), without a cursor
    pub fn text(&self, text: &str, edge: Edge) -> Frag {
        Frag::new_simple(self.make_node(DocNode::Text(text.to_string())), edge, None)
    }

    /// The source text of `node` unchanged, comments included, with the edges of
    /// its first and last token; its last token is the cursor
    pub fn verbatim(&self, node: &SyntaxNode, start: Edge, end: Edge) -> Frag {
        let (first, last) = (node.first_token().unwrap(), node.last_token().unwrap());
        self.verbatim_range(&first, &last, start, end)
    }

    /// The source text from `first` to `last` (inclusive) unchanged, see `verbatim`
    pub fn verbatim_range(
        &self,
        first: &SyntaxToken,
        last: &SyntaxToken,
        start: Edge,
        end: Edge,
    ) -> Frag {
        let mut text = String::new();
        let mut token = Some(first.clone());
        while let Some(t) = token {
            text.push_str(t.text());
            if &t == last {
                break;
            }
            token = t.next_token();
        }
        Frag::new(
            self.make_node(DocNode::Text(text)),
            start,
            end,
            Some(last.clone()),
        )
    }

    /// The source text from `first` to `last` unchanged (see `verbatim_range`), recorded as text that
    /// cannot be formatted yet because of `what` (see `take_unsupported`)
    pub fn unsupported(&self, first: &SyntaxToken, last: &SyntaxToken, what: String) -> Frag {
        let range = TextRange::new(first.text_range().start(), last.text_range().end());
        self.unsupported
            .borrow_mut()
            .push(Unsupported { range, what });
        self.verbatim_range(first, last, Word, Word)
    }

    /// The text recorded by `unsupported`, in the order it was copied
    pub fn take_unsupported(&self) -> Vec<Unsupported> {
        self.unsupported.take()
    }

    /// Just the token, without a cursor: the caller places the trivia (comments) that follow it
    pub fn token_without_cursor(&self, token: SyntaxToken, edge: Edge) -> Frag {
        Frag::new_simple(self.make_node(DocNode::Token(token)), edge, None)
    }

    /// Keyword that is always followed by a space, e.g. `let (a, b)`
    pub fn keyword(&self, token: SyntaxToken) -> Frag {
        Frag::new(
            self.make_node(DocNode::Token(token.clone())),
            Word,
            Keyword,
            Some(token),
        )
    }

    fn concat(&self, doc1: DocId, doc2: DocId) -> DocId {
        if doc1 == EMPTY_DOC {
            return doc2;
        }
        if doc2 == EMPTY_DOC {
            return doc1;
        }
        self.make_node(DocNode::Concat(doc1, doc2))
    }

    pub fn nest(&self, frag: Frag) -> Frag {
        self.map_branches(frag, |b| Branch {
            doc_id: self.make_node(DocNode::Nest(b.doc_id)),
            ..b
        })
    }

    /// Puts `sep` (a line break, or a space or a line break) in front of every branch; the fragment
    /// then starts with `start` (`LineStart` or `Nothing`), so no space is computed in front of it
    fn prefix_break(&self, sep: DocId, start: Edge, frag: Frag) -> Frag {
        self.map_branches(frag, |b| Branch {
            start,
            doc_id: self.concat(sep, b.doc_id),
            ..b
        })
    }

    /// The following lines of the fragment are indented to the column where it starts (visual
    /// indentation), e.g. closure parameters: `|a: A,` / ` b: B|`
    pub fn align(&self, frag: Frag) -> Frag {
        self.map_branches(frag, |b| Branch {
            doc_id: self.make_node(DocNode::Align(b.doc_id)),
            ..b
        })
    }

    /// The fragment starts on a new line with the current indentation
    pub fn on_next_line(&self, frag: Frag) -> Frag {
        self.prefix_break(NEWLINE_DOC, LineStart, frag)
    }

    /// The fragment continues the current line after a space, or starts a new line
    pub fn on_same_or_next_line(&self, frag: Frag) -> Frag {
        self.prefix_break(NEWLINE_OR_SPACE_DOC, Nothing, frag)
    }

    /// The fragment starts on a new line, indented one level deeper. The newline is inside
    /// the nest, otherwise the resolver measures the first nested line (and aligns multi-line
    /// comments) with the outer indent.
    pub fn indented_line(&self, frag: Frag) -> Frag {
        self.nest(self.on_next_line(frag))
    }

    /// The body of a block or a broken list after its opening delimiter, indented on the next lines.
    /// With `line_comments` (the body starts with comments ending the delimiter's line, see
    /// `Builder::push_comments_ending_line`), they stay on the delimiter's line after a space if it
    /// fits, otherwise they are the body's first line.
    pub fn body_after_opener(&self, line_comments: bool, body: Frag) -> Frag {
        self.nest(if line_comments {
            self.on_same_or_next_line(body)
        } else {
            self.on_next_line(body)
        })
    }

    /// The fragment continues the current line, or starts an indented next line,
    /// e.g. the right-hand side of `=`; broken over more lines on the next line, it costs a split,
    /// so it is broken there only if it does not fit on the current line either
    pub fn same_or_indented_line(&self, frag: Frag) -> Frag {
        self.alt_frag(frag.clone(), self.indented_line(self.keep_together(frag)))
    }

    /// The fragment is always followed by a space, e.g. an attribute in front of a parameter:
    /// `#[cfg(x)] &self`
    pub fn followed_by_space(&self, frag: Frag) -> Frag {
        self.map_branches(frag, |b| Branch { end: Keyword, ..b })
    }

    /// Restricts the fragment to layouts that span more than one line
    pub fn multi_line(&self, frag: Frag) -> Frag {
        self.map_branches(frag, |b| Branch {
            doc_id: self.make_node(DocNode::MultiLine(b.doc_id)),
            ..b
        })
    }

    /// `first` followed by `second`, which breaks whenever `first` does: either `first` is on a single
    /// line, or both span more lines, e.g. broken generics of a function followed by its parameters
    /// one per line. `second` must have a layout spanning more lines.
    pub fn concat_breaking_together(&self, first: Frag, second: Frag) -> Frag {
        let flat = Builder::concat(self, [self.flat(first.clone()), second.clone()]);
        let broken = Builder::concat(self, [self.multi_line(first), self.multi_line(second)]);
        self.alt_frag(flat, broken)
    }

    /// The fragment should stay on one line if possible: a layout spanning more lines is used only when
    /// all single-line layouts overflow, e.g. a type
    pub fn keep_together(&self, frag: Frag) -> Frag {
        self.map_branches(frag, |b| Branch {
            doc_id: self.make_node(DocNode::KeepTogether(b.doc_id, Cost::new(1))),
            ..b
        })
    }

    /// The fragment costs a split, so it is used only when the layouts without it are worse: a layout
    /// the Style Guide accepts only as a fallback, e.g. `{` on its own line after an `if` head
    pub fn dispreferred(&self, frag: Frag) -> Frag {
        self.map_branches(frag, |b| Branch {
            doc_id: self.make_node(DocNode::Dispreferred(b.doc_id, Cost::new(1))),
            ..b
        })
    }

    /// Restricts the fragment to layouts whose last line ends at most at the indentation of a nested
    /// line, so a following fragment on that line still starts left of the nested lines below it,
    /// e.g. a chain root followed by the first element: `x.baz?` / `    .qux()`
    pub fn short_last_line(&self, frag: Frag) -> Frag {
        self.map_branches(frag, |b| Branch {
            doc_id: self.make_node(DocNode::ShortLastLine(b.doc_id)),
            ..b
        })
    }

    /// Restricts the fragment to layouts on a single line
    pub fn flat(&self, frag: Frag) -> Frag {
        self.map_branches(frag, |b| Branch {
            doc_id: self.make_node(DocNode::Flat(b.doc_id)),
            ..b
        })
    }

    /// Single-line layout of `frag`, if its width is at most `max_width`
    pub fn flat_within(&self, frag: Frag, max_width: u32) -> Option<Frag> {
        self.flat_width(&frag)
            .is_some_and(|size| size as u32 <= max_width)
            .then(|| self.flat(frag))
    }

    fn alt_doc(&self, doc1: DocId, doc2: DocId) -> DocId {
        if doc1 == doc2 {
            return doc1;
        }
        self.make_node(DocNode::Alt(doc1, doc2))
    }

    pub fn alt_opt(&self, frag: Frag, other: Option<Frag>) -> Frag {
        match other {
            Some(other) => self.alt_frag(frag, other),
            None => frag,
        }
    }

    /// The alternative of the fragments that are present
    pub fn alt_some(&self, frag1: Option<Frag>, frag2: Option<Frag>) -> Option<Frag> {
        match (frag1, frag2) {
            (Some(frag1), frag2) => Some(self.alt_opt(frag1, frag2)),
            (None, frag2) => frag2,
        }
    }

    pub fn alt_frag(&self, frag1: Frag, frag2: Frag) -> Frag {
        // The alternatives cover the same source, so they end at the same cursor; the trivia after it
        // (comments, empty lines) cannot depend on the chosen branch
        debug_assert_eq!(frag1.cursor, frag2.cursor);
        let mut branches = frag1.branches;
        branches.extend(frag2.branches);
        Frag {
            branches: self.group_branches(branches),
            cursor: frag1.cursor,
        }
    }

    fn map_branches(&self, mut frag: Frag, f: impl FnMut(Branch) -> Branch) -> Frag {
        let branches = frag.branches.into_iter().map(f).collect();
        frag.branches = self.group_branches(branches);
        frag
    }

    /// Merges branches with the same edges into a single branch
    fn group_branches(&self, branches: Branches) -> Branches {
        if branches.len() <= 1 {
            return branches;
        }
        let mut out = Branches::new();
        for b in branches {
            if let Some(o) = out
                .iter_mut()
                .find(|o| o.start == b.start && o.end == b.end)
            {
                o.doc_id = self.alt_doc(o.doc_id, b.doc_id);
            } else {
                out.push(b);
            }
        }
        out
    }

    /// Doc of the whole fragment; the edges are dropped
    fn frag_to_doc(&self, frag: &Frag) -> DocId {
        frag.branches
            .iter()
            .map(|b| b.doc_id)
            .reduce(|a, b| self.alt_doc(a, b))
            .unwrap()
    }

    pub fn dump(&self, frag: &Frag) {
        let root = self.frag_to_doc(frag);
        self.doc_arena.borrow().dump(root);
    }

    /// The doc graph of the fragment in the Graphviz DOT format
    pub fn doc_graph_to_dot(&self, frag: &Frag) -> String {
        let root = self.frag_to_doc(frag);
        self.doc_arena.borrow().to_dot(root)
    }

    pub fn into_resolver(self, root: Frag) -> Resolver<'a> {
        let root = self.frag_to_doc(&root);
        Resolver::new(self.doc_arena.into_inner(), self.config, root)
    }

    fn comment_doc(&self, token: &SyntaxToken) -> DocId {
        if let Some(lines) = multiline_comment_lines(token) {
            // The lines are aligned to the column of the comment start
            let mut lines = lines.into_iter();
            let mut doc_id = self.make_node(DocNode::Text(lines.next().unwrap().to_string()));
            for line in lines {
                doc_id = self.concat(doc_id, NEWLINE_DOC);
                if !line.is_empty() {
                    doc_id = self.concat(doc_id, self.make_node(DocNode::Text(line.to_string())));
                }
            }
            return self.make_node(DocNode::Align(doc_id));
        }
        let normalized = self
            .config
            .normalize_comments
            .then(|| normalize_block_comment(token.text()))
            .flatten();
        self.make_node(match normalized {
            Some(text) => DocNode::Text(text),
            None => DocNode::Token(token.clone()),
        })
    }

    /// Width of the doc on a single line, the narrowest of its alternatives; `None` if it cannot be on
    /// a single line, also because of a text with a line break (a multi-line string literal)
    fn flat_width_doc(&self, doc_id: DocId) -> Option<usize> {
        let text_width = |text: &str| {
            (!text.contains('\n')).then(|| unicode_width::UnicodeWidthStr::width(text))
        };
        match self.doc_arena.borrow().get_node(doc_id) {
            DocNode::Token(token) => text_width(token.text()),
            DocNode::Text(t) => text_width(t),
            Newline | DocNode::Nest(_) | DocNode::Align(_) => None,
            DocNode::Flat(d) => self.flat_width_doc(*d),
            DocNode::MultiLine(_) => None,
            DocNode::KeepTogether(d, _) | DocNode::Dispreferred(d, _) | DocNode::ShortLastLine(d) => {
                self.flat_width_doc(*d)
            }
            DocNode::Concat(a, b) => self
                .flat_width_doc(*a)
                .and_then(|size1| self.flat_width_doc(*b).map(|size2| size1 + size2)),
            // Width of the narrowest single-line branch
            DocNode::Alt(a, b) => match (self.flat_width_doc(*a), self.flat_width_doc(*b)) {
                (Some(size1), Some(size2)) => Some(size1.min(size2)),
                (size1, size2) => size1.or(size2),
            },
        }
    }

    /// Width of the fragment on a single line, the widest of its branches that can be on a single line
    /// (each with its narrowest alternative); `None` if no branch can be (a branch that cannot is
    /// dropped by `flat`, e.g. the braced layout of a closure)
    pub fn flat_width(&self, frag: &Frag) -> Option<usize> {
        frag.branches
            .iter()
            .filter_map(|b| self.flat_width_doc(b.doc_id))
            .max()
    }
}
