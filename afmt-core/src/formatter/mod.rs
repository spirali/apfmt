//! Building docs: the `Formatter` (doc arena and combinators), fragments (`Frag`) and the `Builder`
//! that glues fragments together and places the source trivia (comments, blank lines)

mod builder;
mod frag;
mod trivia;

use crate::comments::{is_doc_comment, tokens_between};
use crate::config::Config;
use crate::doc::{
    AlignGroup, Cost, DocArena, DocId, DocNode, EMPTY_DOC, LINE_COST, NEWLINE_DOC,
    NEWLINE_OR_SPACE_DOC, SPLIT_COST, WeakPreference,
};
use crate::formatter::Edge::{Keyword, LineStart, Nothing, Word};
use crate::resolver::Resolver;
use crate::syntax_utils::{Placeholders, Reparsed};
pub(crate) use builder::Builder;
use frag::{Branch, Branches};
pub(crate) use frag::{Edge, Frag};
use ra_ap_syntax::{SyntaxNode, SyntaxToken, TextRange};
use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use trivia::{multiline_comment_lines, normalize_comment_sigil_spacing, trim_comment_line};

/// The cost of `Formatter::overflow_fallback`: more splits than any layout of real code has,
/// and small enough that adding many of them does not saturate
const OVERFLOW_FALLBACK: Cost = Cost::splits(10_000);

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
    align_group_id_counter: Cell<AlignGroup>,
    /// The source ranges of the nodes recorded by `mark_placed` (debug builds only)
    placed: RefCell<HashSet<TextRange>>,
    /// The placeholders of the nodes parsed again that are being formatted (see `format_reparsed`)
    placeholders: RefCell<Placeholders>,
}

impl<'a> Formatter<'a> {
    pub fn new(config: &'a Config) -> Self {
        Formatter {
            config,
            doc_arena: RefCell::new(DocArena::new()),
            unsupported: RefCell::new(Vec::new()),
            align_group_id_counter: Cell::new(AlignGroup::new(0)),
            placed: RefCell::new(HashSet::new()),
            placeholders: RefCell::new(Placeholders::new()),
        }
    }

    /// Records in debug builds that `node` is placed in the output, for checks with `was_placed`
    pub fn mark_placed(&self, node: &SyntaxNode) {
        if cfg!(debug_assertions) {
            self.placed.borrow_mut().insert(node.text_range());
        }
    }

    /// Whether `node` was recorded by `mark_placed`; always true in release builds
    pub fn was_placed(&self, node: &SyntaxNode) -> bool {
        !cfg!(debug_assertions) || self.placed.borrow().contains(&node.text_range())
    }

    /// `None` if the alignment of trailing comments is disabled
    pub fn align_group(&self) -> Option<AlignGroup> {
        if self.config.trailing_comment_align_threshold == 0 {
            return None;
        }
        let mut counter = self.align_group_id_counter.get();
        let new_id = counter.bump();
        self.align_group_id_counter.set(counter);
        Some(new_id)
    }

    #[inline]
    pub fn config(&self) -> &'a Config {
        self.config
    }

    #[inline]
    fn make_node(&self, doc_node: DocNode) -> DocId {
        self.doc_arena.borrow_mut().make(doc_node)
    }

    #[inline]
    /// A `Text` node of `text`
    fn text_node(&self, text: impl Into<Box<str>>) -> DocId {
        self.doc_arena.borrow_mut().make_text(text)
    }

    /// The node parsed again by `reparse` (given the placeholders in effect, e.g. `|placeholders|
    /// parse_token_tree(tree, edition, placeholders)`) formatted by `format`; while it runs, its
    /// placeholders are written as the text they stand for (`$x`, see `token_text`), and they are
    /// dropped after, since the docs have the text. `None` if `reparse` does not parse the node.
    pub fn format_reparsed<N>(
        &self,
        reparse: impl FnOnce(&Placeholders) -> Option<Reparsed<N>>,
        format: impl FnOnce(N) -> Frag,
    ) -> Option<Frag> {
        let Reparsed { node, placeholders } = reparse(&self.placeholders.borrow())?;
        let added: Vec<_> = placeholders
            .iter()
            .map(|(token, _)| token.clone())
            .collect();
        self.placeholders.borrow_mut().extend(placeholders);
        let frag = format(node);
        let mut placeholders = self.placeholders.borrow_mut();
        for token in &added {
            placeholders.remove(token);
        }
        Some(frag)
    }

    /// The text a token is written as: its own, or the text of a placeholder (see `format_reparsed`)
    fn token_text<'t>(&self, token: &'t SyntaxToken) -> Cow<'t, str> {
        match self.placeholders.borrow().get(token) {
            Some(text) => Cow::Owned(text.clone()),
            None => Cow::Borrowed(token.text()),
        }
    }

    /// The doc of a token: the token, or the text it is written as (see `format_reparsed`)
    fn token_node(&self, token: SyntaxToken) -> DocId {
        let placeholders = self.placeholders.borrow();
        match placeholders.get(&token) {
            Some(text) => self.text_node(text.as_str()),
            None => self.make_node(DocNode::Token(token)),
        }
    }

    pub fn token(&self, token: SyntaxToken, edge: Edge) -> Frag {
        Frag::new_simple(self.token_node(token.clone()), edge, Some(token))
    }

    /// A token with different edges on its sides, e.g. the negative `!` of `impl !Send`: a space in
    /// front of it (`Word`), none after it (`Tight`)
    pub fn token_with_edges(&self, token: SyntaxToken, start: Edge, end: Edge) -> Frag {
        Frag::new(self.token_node(token.clone()), start, end, Some(token))
    }

    /// A postfix operator, `?` or a `..` without an end: no space in front of it, spaced like a word
    /// after it (`x? == y`, `x? as u8`, `2.. =>`, not `2..=>`), but tight to `.`, `?`, `(`, `[`
    pub fn postfix_token(&self, token: SyntaxToken) -> Frag {
        self.token_with_edges(token, Edge::Tight, Edge::Word)
    }

    /// Text that is not in the source (e.g. an inserted `"C"`), without a cursor
    pub fn text(&self, text: &str, edge: Edge) -> Frag {
        Frag::new_simple(self.text_node(text), edge, None)
    }

    /// The source text from `first` to `last` (inclusive), its lines after the first shifted by the
    /// change of indentation of the line it starts on (`reindented_lines`); the text unchanged (see
    /// `verbatim_range`) when they cannot be
    pub fn reindented_range(
        &self,
        first: &SyntaxToken,
        last: &SyntaxToken,
        start: Edge,
        end: Edge,
    ) -> Frag {
        let Some(lines) = self.reindented_lines(first, last) else {
            return self.verbatim_range(first, last, start, end);
        };
        Frag::new(self.lines_doc(lines), start, end, Some(last.clone()))
    }

    /// The lines one below another, each at the indentation of the doc; a blank line is empty
    fn lines_doc<S: AsRef<str> + Into<Box<str>>>(
        &self,
        lines: impl IntoIterator<Item = S>,
    ) -> DocId {
        let mut lines = lines.into_iter();
        let mut doc_id = lines.next().map_or(EMPTY_DOC, |line| self.text_node(line));
        for line in lines {
            doc_id = self.concat(doc_id, NEWLINE_DOC);
            if !line.as_ref().is_empty() {
                doc_id = self.concat(doc_id, self.text_node(line));
            }
        }
        doc_id
    }

    /// The source text from `first` to `last` (inclusive) unchanged, comments included, with the
    /// edges of its first and last token; its last token is the cursor
    pub fn verbatim_range(
        &self,
        first: &SyntaxToken,
        last: &SyntaxToken,
        start: Edge,
        end: Edge,
    ) -> Frag {
        let text: String = (tokens_between(first, last))
            .map(|t| self.token_text(&t).into_owned())
            .collect();
        Frag::new(self.text_node(text), start, end, Some(last.clone()))
    }

    /// The source text from `first` to `last` reindented (see `reindented_range`), recorded as text that
    /// cannot be formatted yet because of `what` (see `take_unsupported`)
    pub fn unsupported(&self, first: &SyntaxToken, last: &SyntaxToken, what: String) -> Frag {
        let range = TextRange::new(first.text_range().start(), last.text_range().end());
        self.unsupported
            .borrow_mut()
            .push(Unsupported { range, what });
        self.reindented_range(first, last, Word, Word)
    }

    /// The text recorded by `unsupported`, in the order it was copied
    pub fn take_unsupported(&self) -> Vec<Unsupported> {
        self.unsupported.take()
    }

    /// Just the token, without a cursor: the caller places the trivia (comments) that follow it
    pub fn token_without_cursor(&self, token: SyntaxToken, edge: Edge) -> Frag {
        Frag::new_simple(self.token_node(token), edge, None)
    }

    /// Keyword that is always followed by a space, e.g. `let (a, b)`
    pub fn keyword(&self, token: SyntaxToken) -> Frag {
        Frag::new(self.token_node(token.clone()), Word, Keyword, Some(token))
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

    /// The fragment continues the current line, or starts an indented next line, e.g. the right-hand
    /// side of `=`. In the order of preference (Style Guide `let`: "try splitting after the `=`, if the
    /// declaration fits on two lines"): on a single line on the current line, on a single line on the
    /// next line, broken on the current line (a split), broken on the next line (two splits).
    pub fn same_or_indented_line(&self, frag: Frag) -> Frag {
        let (same, next) = self.same_and_indented_line(frag);
        self.alt_frag(same, next)
    }

    /// As `same_or_indented_line`, but broken on the next line costs a line and a half more than
    /// broken on the current line instead of a split: it wins when it saves at least two lines, e.g.
    /// the right-hand side of `let` and of an assignment (as rustfmt's `prefer_next_line`, which
    /// counts the lines of the right-hand side: one less in total is enough for it, which moves
    /// `x = cmd` / `.args([` to `x =` / `cmd.args([` and `]).output()?` here)
    pub fn same_or_indented_line_saving_lines(&self, frag: Frag) -> Frag {
        let next = Cost::new(SPLIT_COST + LINE_COST * 3 / 2);
        let (same, next) = self.same_and_indented_line_with(frag, next);
        self.alt_frag(same, next)
    }

    /// The two layouts of `same_or_indented_line`: on the current line, and on the next line
    pub fn same_and_indented_line(&self, frag: Frag) -> (Frag, Frag) {
        self.same_and_indented_line_with(frag, Cost::splits(2))
    }

    /// The layouts of `same_and_indented_line`, broken on the next line costing `next`
    fn same_and_indented_line_with(&self, frag: Frag, next: Cost) -> (Frag, Frag) {
        let same = self.keep_together_with(frag.clone(), Cost::SPLIT);
        let next = self.keep_together_with(frag, next);
        (same, self.indented_line(next))
    }

    /// The fragment continues the current line, broken there if needed, and starts an indented next
    /// line only when every layout on the current line overflows (`overflow_fallback`), e.g. the value
    /// of a struct literal field (as rustfmt does): `name:` / `    long::path::Value`
    pub fn same_or_indented_line_on_overflow(&self, frag: Frag) -> Frag {
        let next = self.indented_line(self.keep_together(frag.clone()));
        self.alt_frag(frag, self.overflow_fallback(next))
    }

    /// The fragment is used only when every layout without it overflows `max_width`: it costs more
    /// splits than any layout of real code has (the resolver compares the overflow first)
    pub fn overflow_fallback(&self, frag: Frag) -> Frag {
        self.cost(frag, OVERFLOW_FALLBACK)
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

    /// `first` followed by `second`, which spans more lines only when `first` does: both on a single
    /// line, or `first` broken, e.g. the parameters of a function before its return type
    /// (`fn f(` / `    a: A,` / `) -> Result<` / `...`). `first` must have a layout spanning more lines.
    pub fn concat_breaking_in_order(&self, first: Frag, second: Frag) -> Frag {
        let flat = Builder::concat(self, [self.flat(first.clone()), self.flat(second.clone())]);
        let broken = Builder::concat(self, [self.multi_line(first), second]);
        self.alt_frag(flat, broken)
    }

    /// `first` followed by `second` on the next line when `first` is on a single line, or right after
    /// the last line of `first` when it spans more lines, e.g. the `where` clause of a function after
    /// its signature (`fn f(a: A)` / `where`, or `fn f(` / `    a: A,` / `) where`)
    pub fn concat_on_next_line_unless_broken(&self, first: Frag, second: Frag) -> Frag {
        let flat = Builder::concat(self, [self.flat(first.clone()), self.on_next_line(second.clone())]);
        let broken = Builder::concat(self, [self.multi_line(first), second]);
        self.alt_frag(flat, broken)
    }

    /// The fragment should stay on one line if possible: a layout spanning more lines is used only when
    /// all single-line layouts overflow, e.g. a type
    pub fn keep_together(&self, frag: Frag) -> Frag {
        self.keep_together_with(frag, Cost::SPLIT)
    }

    /// `keep_together` with the layouts spanning more lines getting `cost`
    fn keep_together_with(&self, frag: Frag, cost: Cost) -> Frag {
        self.map_branches(frag, |b| Branch {
            doc_id: self.make_node(DocNode::KeepTogether(b.doc_id, cost)),
            ..b
        })
    }

    /// The fragment costs a split, so it is used only when the layouts without it are worse: a layout
    /// the Style Guide accepts only as a fallback, e.g. `{` on its own line after an `if` head
    pub fn dispreferred(&self, frag: Frag) -> Frag {
        self.cost(frag, Cost::SPLIT)
    }

    /// Every layout of the fragment gets the cost, compared right after the overflow (see
    /// `DocNode::Cost`)
    pub fn cost(&self, frag: Frag, cost: Cost) -> Frag {
        self.map_branches(frag, |b| Branch {
            doc_id: self.make_node(DocNode::Cost(b.doc_id, cost)),
            ..b
        })
    }

    /// The fragment gets the preference, compared only after the cost (the lines): it decides between
    /// layouts that are otherwise equal, e.g. a call whose single argument overflows rather than a
    /// broken one in as many lines
    pub fn weak_preference(&self, frag: Frag, preference: WeakPreference) -> Frag {
        self.map_branches(frag, |b| Branch {
            doc_id: self.make_node(DocNode::WeakPreference(b.doc_id, preference)),
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

    /// Restricts the fragment to layouts whose first line spans at most `width` columns from where it
    /// starts (see `DocNode::FirstLineWithin`)
    pub fn first_line_within(&self, frag: Frag, width: u32) -> Frag {
        self.map_branches(frag, |b| Branch {
            doc_id: self.make_node(DocNode::FirstLineWithin(b.doc_id, width)),
            ..b
        })
    }

    /// The doc does not count as overflow beyond the page width (see `DocNode::IgnoreOverflow`)
    pub(super) fn ignore_overflow(&self, doc_id: DocId) -> DocId {
        self.make_node(DocNode::IgnoreOverflow(doc_id))
    }

    /// Restricts the fragment to layouts on a single line
    pub fn flat(&self, frag: Frag) -> Frag {
        self.map_branches(frag, |b| Branch {
            doc_id: self.make_node(DocNode::Flat(b.doc_id)),
            ..b
        })
    }

    /// Single-line layout of `frag`, if it has one (see `has_flat_layout`)
    pub fn flat_if_possible(&self, frag: Frag) -> Option<Frag> {
        self.has_flat_layout(&frag).then(|| self.flat(frag))
    }

    /// The fragment on a single line where it fits without overflow with `room` columns left after it
    /// (`DocNode::RoomAfter`), for text that always follows on the line; in any of its layouts only
    /// where it does not (`DocNode::Fallback`), whatever what follows could gain by breaking it. A
    /// branch without a single-line layout (see `has_flat_layout`) is kept as it is.
    pub fn flat_where_fits(&self, frag: Frag, room: u32) -> Frag {
        self.map_branches(frag, |b| {
            if !self.doc_arena.borrow_mut().has_flat_layout(b.doc_id) {
                return b;
            }
            let flat = self.make_node(DocNode::Flat(b.doc_id));
            let fits = self.make_node(DocNode::RoomAfter(flat, room));
            Branch {
                doc_id: self.make_node(DocNode::Fallback(fits, b.doc_id)),
                ..b
            }
        })
    }

    /// Whether a branch of the fragment satisfies `query`, a walk of `DocArena`
    fn any_branch(&self, frag: &Frag, query: impl Fn(&mut DocArena, DocId) -> bool) -> bool {
        let mut arena = self.doc_arena.borrow_mut();
        frag.branches.iter().any(|b| query(&mut arena, b.doc_id))
    }

    /// Whether a branch of the fragment may have a layout under `Flat`: false only if a line break
    /// is forced in all of them, or a width limit in it cannot be met (see
    /// `DocArena::has_flat_layout`)
    pub fn has_flat_layout(&self, frag: &Frag) -> bool {
        self.any_branch(frag, DocArena::has_flat_layout)
    }

    /// The width of the narrowest single line of the fragment without any line break (see
    /// `DocArena::min_flat_width`; of its branches, the widest), `None` if no branch has one: whether
    /// it can be on one line within a width, for a decision on the layouts to build; the resolver
    /// measures the layout it places
    pub fn min_flat_width(&self, frag: &Frag) -> Option<u32> {
        let mut arena = self.doc_arena.borrow_mut();
        frag.branches
            .iter()
            .filter_map(|b| arena.min_flat_width(b.doc_id))
            .max()
    }

    /// The narrowest first line of the fragment in a layout with a line break that takes no
    /// dispreferred choice up to it (see `DocArena::natural_first_line_width`; of its branches, the
    /// widest), `None` if no branch has one
    pub fn natural_first_line_width(&self, frag: &Frag) -> Option<u32> {
        let mut arena = self.doc_arena.borrow_mut();
        frag.branches
            .iter()
            .filter_map(|b| arena.natural_first_line_width(b.doc_id))
            .max()
    }

    /// Whether a text of the fragment contains a line break, e.g. a multi-line string literal: a
    /// layout with it on a single line still spans more lines
    pub fn has_multi_line_text(&self, frag: &Frag) -> bool {
        self.any_branch(frag, DocArena::has_multi_line_text)
    }

    /// Whether a layout of the fragment may span more lines (see `DocArena::has_line_break`)
    pub fn has_line_break(&self, frag: &Frag) -> bool {
        self.any_branch(frag, DocArena::has_line_break)
    }

    /// Single-line layout of `frag` at most `max_width` wide (the resolver measures it, see
    /// `DocNode::FirstLineWithin`), without a line break even inside a token (a multi-line string
    /// literal, which `flat` allows; `u32::MAX` for no other limit); `None` if even its narrowest
    /// single line is wider (see `min_flat_width`), so it is not built only to be ruled out
    pub fn flat_within(&self, frag: Frag, max_width: u32) -> Option<Frag> {
        let fits = |arena: &mut DocArena, doc_id| {
            arena.min_flat_width(doc_id).is_some_and(|w| w <= max_width)
        };
        self.any_branch(&frag, fits)
            .then(|| self.flat(self.first_line_within(frag, max_width)))
    }

    /// The alternative of two docs
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

    fn align_mark(&self, group: AlignGroup) -> DocId {
        self.make_node(DocNode::AlignMark(group))
    }

    fn comment_doc(&self, token: &SyntaxToken) -> DocId {
        let doc = is_doc_comment(token.text());
        if let Some(lines) = multiline_comment_lines(token) {
            // The lines are aligned to the column of the comment start
            let lines = lines.into_iter().map(|line| trim_comment_line(line, doc));
            return self.make_node(DocNode::Align(self.lines_doc(lines)));
        }
        let source = token.text();
        // Allocated only when the text changes
        let trimmed: Cow<str> = if source.contains('\n') {
            let lines: Vec<_> = source
                .split('\n')
                .map(|line| trim_comment_line(line, doc))
                .collect();
            lines.join("\n").into()
        } else {
            trim_comment_line(source, doc).into()
        };
        let normalized = (self.config.normalize_comment_sigil_spacing && !doc)
            .then(|| normalize_comment_sigil_spacing(&trimmed))
            .flatten();
        let text = normalized.map_or(trimmed, Cow::Owned);
        if text == source {
            self.make_node(DocNode::Token(token.clone()))
        } else {
            self.text_node(text)
        }
    }
}
