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
use rustc_hash::FxHashSet;
use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use trivia::{multiline_comment_lines, normalize_comment_sigil_spacing, trim_comment_line};

/// More splits than any layout of real code has, small enough not to saturate when added up
const OVERFLOW_FALLBACK: Cost = Cost::splits(10_000);

/// Source text that apfmt cannot format yet, copied unchanged (see `Formatter::unsupported`)
/// How many lines a layout broken on the next line must save to win over one broken on the
/// current line
#[derive(Clone, Copy)]
pub(crate) enum NextLine {
    /// The order of the Style Guide (`let`): single line on the current line, single line on the
    /// next line, broken on the current line, broken on the next line
    InOrder,
    /// rustfmt needs one: `x = cmd` / `.args([` → `x =` / `cmd.args([`
    SavingTwoLines,
    SavingOneLine,
}

impl NextLine {
    fn cost(self) -> Cost {
        match self {
            NextLine::InOrder => Cost::splits(2),
            NextLine::SavingTwoLines => Cost::new(SPLIT_COST + LINE_COST * 3 / 2),
            NextLine::SavingOneLine => Cost::new(SPLIT_COST + LINE_COST / 2),
        }
    }
}

pub(crate) struct Unsupported {
    pub range: TextRange,
    /// What is not supported, e.g. `MACRO_EXPR`
    pub what: String,
}

pub(crate) struct Formatter<'a> {
    doc_arena: RefCell<DocArena>,
    config: &'a Config,
    unsupported: RefCell<Vec<Unsupported>>,
    align_group_id_counter: Cell<AlignGroup>,
    /// Debug builds only
    placed: RefCell<FxHashSet<TextRange>>,
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
            placed: RefCell::new(FxHashSet::default()),
            placeholders: RefCell::new(Placeholders::default()),
        }
    }

    /// Debug builds only, for checks with `was_placed`
    pub fn mark_placed(&self, node: &SyntaxNode) {
        if cfg!(debug_assertions) {
            self.placed.borrow_mut().insert(node.text_range());
        }
    }

    /// Always true in release builds
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
    fn text_node(&self, text: impl Into<Box<str>>) -> DocId {
        self.doc_arena.borrow_mut().make_text(text)
    }

    /// The node parsed again by `reparse` formatted by `format`; meanwhile its placeholders are
    /// written as the text they stand for (`$x`). `None` if `reparse` does not parse the node.
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

    pub fn can_reparse<N>(
        &self,
        reparse: impl FnOnce(&Placeholders) -> Option<Reparsed<N>>,
    ) -> bool {
        reparse(&self.placeholders.borrow()).is_some()
    }

    fn token_text<'t>(&self, token: &'t SyntaxToken) -> Cow<'t, str> {
        match self.placeholders.borrow().get(token) {
            Some(text) => Cow::Owned(text.clone()),
            None => Cow::Borrowed(token.text()),
        }
    }

    /// The placeholder of a repetition of `macro_rules!`, `$( ... ) sep? op`, which may end with
    /// its own separator or expand to nothing
    pub fn is_repetition_placeholder(&self, token: &SyntaxToken) -> bool {
        (self.placeholders.borrow().get(token)).is_some_and(|text| text.starts_with("$("))
    }

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

    /// E.g. the negative `!` of `impl !Send`: a space in front of it, none after it
    pub fn token_with_edges(&self, token: SyntaxToken, start: Edge, end: Edge) -> Frag {
        Frag::new(self.token_node(token.clone()), start, end, Some(token))
    }

    /// `?` or a `..` without an end: tight in front, spaced like a word after it (`x? as u8`, `2..
    /// =>`)
    pub fn postfix_token(&self, token: SyntaxToken) -> Frag {
        self.token_with_edges(token, Edge::Tight, Edge::Word)
    }

    /// Text that is not in the source (e.g. an inserted `"C"`), without a cursor
    pub fn text(&self, text: &str, edge: Edge) -> Frag {
        Frag::new_simple(self.text_node(text), edge, None)
    }

    /// The source text from `first` to `last`, its lines after the first shifted by the change of
    /// indentation of its first line; unchanged when they cannot be
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

    /// The source text from `first` to `last` unchanged, comments included
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

    /// The source reindented, recorded as unsupported because of `what` (see `take_unsupported`)
    pub fn unsupported(&self, first: &SyntaxToken, last: &SyntaxToken, what: String) -> Frag {
        let range = TextRange::new(first.text_range().start(), last.text_range().end());
        self.unsupported
            .borrow_mut()
            .push(Unsupported { range, what });
        self.reindented_range(first, last, Word, Word)
    }

    pub fn take_unsupported(&self) -> Vec<Unsupported> {
        self.unsupported.take()
    }

    /// The caller places the trivia that follow the token
    pub fn token_without_cursor(&self, token: SyntaxToken, edge: Edge) -> Frag {
        Frag::new_simple(self.token_node(token), edge, None)
    }

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
        self.wrap(frag, DocNode::Nest)
    }

    /// Puts `sep` in front of every branch; the fragment then starts with `start`
    fn prefix_break(&self, sep: DocId, start: Edge, frag: Frag) -> Frag {
        self.map_branches(frag, |b| Branch {
            start,
            doc_id: self.concat(sep, b.doc_id),
            ..b
        })
    }

    /// Visual indentation: the following lines are aligned to the start, e.g. `|a: A,` / ` b: B|`
    pub fn align(&self, frag: Frag) -> Frag {
        self.wrap(frag, DocNode::Align)
    }

    pub fn on_next_line(&self, frag: Frag) -> Frag {
        self.prefix_break(NEWLINE_DOC, LineStart, frag)
    }

    pub fn on_same_or_next_line(&self, frag: Frag) -> Frag {
        self.prefix_break(NEWLINE_OR_SPACE_DOC, Nothing, frag)
    }

    /// The newline is inside the nest, otherwise the resolver measures the first nested line with
    /// the outer indent
    pub fn indented_line(&self, frag: Frag) -> Frag {
        self.nest(self.on_next_line(frag))
    }

    /// The body after an opening delimiter, indented on the next lines. With `line_comments`, the
    /// comments ending the delimiter's line stay there if they fit.
    pub fn body_after_opener(&self, line_comments: bool, body: Frag) -> Frag {
        self.nest(if line_comments {
            self.on_same_or_next_line(body)
        } else {
            self.on_next_line(body)
        })
    }

    /// The current line or the next line, indented; `next` decides when broken on the next line
    /// wins over broken on the current line
    pub fn same_or_indented_line(&self, frag: Frag, next: NextLine) -> Frag {
        let (same, next) = self.same_and_indented_line(frag, next);
        self.alt_frag(same, next)
    }

    /// As `same_or_indented_line` with `NextLine::SavingTwoLines`, but `preferred` on the next line
    /// wins when it saves one line, as in rustfmt (`x =` / `a.b().c(|y| {` against `x = a` /
    /// `.b()` / `.c(|y| {`)
    pub fn same_or_indented_line_preferring(&self, other: Frag, preferred: Frag) -> Frag {
        let all = self.alt_frag(other.clone(), preferred.clone());
        let same = self.keep_together_with(all, Cost::SPLIT);
        let other = self.keep_together_with(other, NextLine::SavingTwoLines.cost());
        let preferred = self.keep_together_with(preferred, NextLine::SavingOneLine.cost());
        self.alt_frag(same, self.indented_line(self.alt_frag(other, preferred)))
    }

    pub fn same_and_indented_line(&self, frag: Frag, next: NextLine) -> (Frag, Frag) {
        let same = self.keep_together_with(frag.clone(), Cost::SPLIT);
        let next = self.keep_together_with(frag, next.cost());
        (same, self.indented_line(next))
    }

    /// On the next line only when every layout on the current line overflows, e.g. a struct literal
    /// field value (as rustfmt): `name:` / `    long::path::Value`
    pub fn same_or_indented_line_on_overflow(&self, frag: Frag) -> Frag {
        let next = self.indented_line(self.keep_together(frag.clone()));
        self.alt_frag(frag, self.overflow_fallback(next))
    }

    /// Used only when every layout without it overflows `max_width`
    pub fn overflow_fallback(&self, frag: Frag) -> Frag {
        self.cost(frag, OVERFLOW_FALLBACK)
    }

    /// E.g. `#[cfg(x)] &self`; a layout ending with a line comment still ends the line
    pub fn followed_by_space(&self, frag: Frag) -> Frag {
        self.map_branches(frag, |b| match b.end {
            Edge::Break => b,
            _ => Branch { end: Keyword, ..b },
        })
    }

    pub fn multi_line(&self, frag: Frag) -> Frag {
        self.wrap(frag, DocNode::MultiLine)
    }

    /// `second` breaks whenever `first` does, e.g. broken generics followed by parameters one per
    /// line. `second` must have a layout spanning more lines.
    pub fn concat_breaking_together(&self, first: Frag, second: Frag) -> Frag {
        let flat = Builder::concat(self, [self.flat(first.clone()), second.clone()]);
        let broken = Builder::concat(self, [self.multi_line(first), self.multi_line(second)]);
        self.alt_frag(flat, broken)
    }

    /// `second` spans more lines only when `first` does, e.g. parameters before the return type.
    /// `first` must have a layout spanning more lines.
    pub fn concat_breaking_in_order(&self, first: Frag, second: Frag) -> Frag {
        let flat = Builder::concat(self, [self.flat(first.clone()), self.flat(second.clone())]);
        let broken = Builder::concat(self, [self.multi_line(first), second]);
        self.alt_frag(flat, broken)
    }

    /// `second` on the next line when `first` is flat, otherwise after its last line, e.g. `where`
    /// (`fn f(a: A)` / `where`, or `fn f(` / `    a: A,` / `) where`)
    pub fn concat_on_next_line_unless_broken(&self, first: Frag, second: Frag) -> Frag {
        let flat = Builder::concat(
            self,
            [self.flat(first.clone()), self.on_next_line(second.clone())],
        );
        let broken = Builder::concat(self, [self.multi_line(first), second]);
        self.alt_frag(flat, broken)
    }

    /// When broken, what follows starts on the next line, e.g. `{` after a broken item header
    pub fn ends_line_if_broken(&self, frag: Frag, ends_line: bool) -> Frag {
        if !ends_line {
            return frag;
        }
        let mut broken = Builder::new(self);
        broken.push(self.multi_line(frag.clone()));
        broken.push_newline();
        self.alt_frag(self.flat(frag), broken.into_frag())
    }

    /// A layout spanning more lines is used only when all single-line layouts overflow
    pub fn keep_together(&self, frag: Frag) -> Frag {
        self.keep_together_with(frag, Cost::SPLIT)
    }

    pub fn keep_together_with(&self, frag: Frag, cost: Cost) -> Frag {
        self.wrap(frag, |d| DocNode::KeepTogether(d, cost))
    }

    /// Costs a split: a layout the Style Guide accepts only as a fallback, e.g. `{` on its own line
    pub fn dispreferred(&self, frag: Frag) -> Frag {
        self.cost(frag, Cost::SPLIT)
    }

    pub fn cost(&self, frag: Frag, cost: Cost) -> Frag {
        self.wrap(frag, |d| DocNode::Cost(d, cost))
    }

    /// Compared only after the cost: decides between layouts that are otherwise equal
    pub fn weak_preference(&self, frag: Frag, preference: WeakPreference) -> Frag {
        self.wrap(frag, |d| DocNode::WeakPreference(d, preference))
    }

    /// The last line ends at most at the nested indentation, so what follows starts left of the
    /// nested lines below, e.g. `x.baz?` / `    .qux()`
    pub fn short_last_line(&self, frag: Frag) -> Frag {
        self.wrap(frag, DocNode::ShortLastLine)
    }

    /// Layouts whose last line is only closing delimiters at the starting indentation (`foo(` / `
    /// a,` / `)`). `None` if no layout can end that way.
    pub fn closing_last_line(&self, frag: Frag) -> Option<Frag> {
        let mut arena = self.doc_arena.borrow_mut();
        let branches: Branches = frag
            .branches
            .into_iter()
            .filter(|b| b.end.is_closing())
            .filter(|b| arena.has_line_break(b.doc_id))
            .collect();
        drop(arena);
        if branches.is_empty() {
            return None;
        }
        let frag = Frag {
            branches,
            cursor: frag.cursor,
        };
        Some(self.short_last_line(self.multi_line(frag)))
    }

    pub fn first_line_within(&self, frag: Frag, width: u32) -> Frag {
        self.wrap(frag, |d| DocNode::FirstLineWithin(d, width))
    }

    pub(super) fn ignore_overflow(&self, doc_id: DocId) -> DocId {
        self.make_node(DocNode::IgnoreOverflow(doc_id))
    }

    pub fn flat(&self, frag: Frag) -> Frag {
        self.wrap(frag, DocNode::Flat)
    }

    pub fn flat_if_possible(&self, frag: Frag) -> Option<Frag> {
        self.has_flat_layout(&frag).then(|| self.flat(frag))
    }

    /// Only layouts whose last line leaves `room` columns, for text that always follows on it
    pub fn room_after(&self, frag: Frag, room: u32) -> Frag {
        self.wrap(frag, |d| DocNode::RoomAfter(d, room))
    }

    /// On a single line where it fits with `room` columns left after it, otherwise in any layout,
    /// whatever what follows could gain (`DocNode::Fallback`)
    pub fn flat_where_fits(&self, frag: Frag, room: u32) -> Frag {
        match self.flat_if_possible(frag.clone()) {
            Some(flat) => self.fits_or(flat, room, frag),
            None => frag,
        }
    }

    /// The layouts of `first` that fit with `room` columns left after their last line, otherwise
    /// those of `second` (`DocNode::Fallback`); a branch of `first` pairs with one of `second` by edges
    pub fn fits_or(&self, first: Frag, room: u32, second: Frag) -> Frag {
        self.map_branches(second, |b| {
            let Some(f) = first
                .branches
                .iter()
                .find(|f| f.start == b.start && f.end == b.end)
            else {
                return b;
            };
            let fits = self.make_node(DocNode::RoomAfter(f.doc_id, room));
            Branch {
                doc_id: self.make_node(DocNode::Fallback(fits, b.doc_id)),
                ..b
            }
        })
    }

    fn any_branch(&self, frag: &Frag, query: impl Fn(&mut DocArena, DocId) -> bool) -> bool {
        let mut arena = self.doc_arena.borrow_mut();
        frag.branches.iter().any(|b| query(&mut arena, b.doc_id))
    }

    /// False only if every branch forces a line break or cannot meet a width limit
    pub fn has_flat_layout(&self, frag: &Frag) -> bool {
        self.any_branch(frag, DocArena::has_flat_layout)
    }

    /// The narrowest single line (of the branches, the widest); `None` if a branch has none. For
    /// deciding which layouts to build, not a prediction of the resolver.
    pub fn min_flat_width(&self, frag: &Frag) -> Option<u32> {
        let mut arena = self.doc_arena.borrow_mut();
        frag.branches
            .iter()
            .filter_map(|b| arena.min_flat_width(b.doc_id))
            .max()
    }

    /// The narrowest first line of a broken layout without a dispreferred choice up to it (of the
    /// branches, the widest)
    pub fn natural_first_line_width(&self, frag: &Frag) -> Option<u32> {
        let mut arena = self.doc_arena.borrow_mut();
        frag.branches
            .iter()
            .filter_map(|b| arena.natural_first_line_width(b.doc_id))
            .max()
    }

    /// E.g. a multi-line string literal: a single-line layout still spans more lines
    pub fn has_multi_line_text(&self, frag: &Frag) -> bool {
        self.any_branch(frag, DocArena::has_multi_line_text)
    }

    pub fn has_line_break(&self, frag: &Frag) -> bool {
        self.any_branch(frag, DocArena::has_line_break)
    }

    /// Single-line layout at most `max_width` wide, without a line break even inside a token;
    /// `None` if its narrowest single line is wider, so it is not built only to be ruled out
    pub fn flat_within(&self, frag: Frag, max_width: u32) -> Option<Frag> {
        let fits = |arena: &mut DocArena, doc_id| {
            arena.min_flat_width(doc_id).is_some_and(|w| w <= max_width)
        };
        self.any_branch(&frag, fits)
            .then(|| self.flat(self.first_line_within(frag, max_width)))
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

    pub fn alt_some(&self, frag1: Option<Frag>, frag2: Option<Frag>) -> Option<Frag> {
        match (frag1, frag2) {
            (Some(frag1), frag2) => Some(self.alt_opt(frag1, frag2)),
            (None, frag2) => frag2,
        }
    }

    pub fn alt_frag(&self, frag1: Frag, frag2: Frag) -> Frag {
        // The alternatives cover the same source, so the trivia after their cursor cannot depend on
        // them
        debug_assert_eq!(frag1.cursor, frag2.cursor);
        let mut branches = frag1.branches;
        branches.extend(frag2.branches);
        Frag {
            branches: self.group_branches(branches),
            cursor: frag1.cursor,
        }
    }

    /// Wraps the doc of every branch in `node`, keeping its edges
    fn wrap(&self, frag: Frag, node: impl Fn(DocId) -> DocNode) -> Frag {
        self.map_branches(frag, |b| Branch {
            doc_id: self.make_node(node(b.doc_id)),
            ..b
        })
    }

    fn map_branches(&self, mut frag: Frag, f: impl FnMut(Branch) -> Branch) -> Frag {
        let branches = frag.branches.into_iter().map(f).collect();
        frag.branches = self.group_branches(branches);
        frag
    }

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

    /// The edges are dropped
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

    pub fn doc_graph_to_dot(&self, frag: &Frag) -> String {
        let root = self.frag_to_doc(frag);
        self.doc_arena.borrow().to_dot(root)
    }

    /// Appends the best layout to `out`, then clears the arena: every fragment built before is
    /// invalid
    pub fn render_into(&self, frag: Frag, out: &mut String) {
        let root = self.frag_to_doc(&frag);
        Resolver::new(&self.doc_arena.borrow(), self.config, root).render_into(out);
        self.doc_arena.borrow_mut().clear();
    }

    fn align_mark(&self, group: AlignGroup) -> DocId {
        self.make_node(DocNode::AlignMark(group))
    }

    fn comment_doc(&self, token: &SyntaxToken) -> DocId {
        let doc = is_doc_comment(token.text());
        if let Some(lines) = multiline_comment_lines(token) {
            let lines = lines.into_iter().map(|line| trim_comment_line(line, doc));
            return self.make_node(DocNode::Align(self.lines_doc(lines)));
        }
        let source = token.text();
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
