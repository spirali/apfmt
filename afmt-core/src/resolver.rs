use crate::config::Config;
use crate::content::{Content, ContentArena};
use crate::doc::{AlignGroup, Cost, DocArena, DocId, DocNode, LINE_COST, WeakPreference};
use crate::utils::text_width;
use smallvec::{SmallVec, smallvec};
use std::cmp::Ordering;
use std::collections::HashMap;

#[derive(Clone)]
struct Measure {
    /// The column after the layout; `None` when it ends with a line break: the indentation of the new
    /// line is written with the next text, at the indentation current there (`Renderer`), e.g. the
    /// outer one for `)` after a broken list whose last line break is inside its `Nest`
    last: Option<u32>,
    /// The overflow beyond the page width: the columns beyond it, and `overflow_line_penalty` for each
    /// line that overflows (see `overflow`); saturating, a single line can be very long (e.g. an
    /// embedded blob of data)
    badness: u32,
    /// Compared after `badness`, the lower the better: `LINE_COST` for each line plus the costs
    /// (see `DocNode::Cost`, `DocNode::KeepTogether`)
    cost: i64,
    /// The layout spans more than one line
    multi_line: bool,
    /// Compared after `cost`, the higher the better (see `DocNode::WeakPreference`)
    weak_preference: WeakPreference,
}

fn overflow(col: u32, last: u32, config: &Config) -> u32 {
    let page_width = config.max_width;
    if last <= page_width {
        return 0;
    }
    let penalty = if col <= page_width {
        config.overflow_line_penalty
    } else {
        0
    };
    (last - page_width.max(col)) + penalty
}

impl Measure {
    fn from_text(col: u32, config: &Config, text: &str) -> Self {
        let mut badness = 0u32;
        let mut line_breaks = 0;
        let mut start = col;
        let mut last = col;
        for (i, line) in text.split('\n').enumerate() {
            if i > 0 {
                line_breaks += 1;
                start = 0;
            }
            last = start + text_width(line);
            badness = badness.saturating_add(overflow(start, last, config));
        }
        Measure {
            last: Some(last),
            badness,
            cost: i64::from(line_breaks) * i64::from(LINE_COST),
            multi_line: line_breaks > 0,
            weak_preference: WeakPreference::NEUTRAL,
        }
    }

    fn empty(ctx: &Context) -> Self {
        Measure {
            last: ctx.col,
            badness: 0,
            cost: 0,
            multi_line: false,
            weak_preference: WeakPreference::NEUTRAL,
        }
    }

    /// Whether the layout ranks at least as well as `other` (see `rank_cmp`)
    fn rank_le(&self, other: &Measure) -> bool {
        match self.rank_cmp(other) {
            Ordering::Less | Ordering::Equal => true,
            Ordering::Greater => false,
        }
    }

    /// The order of layouts from the best (`Less`): the overflow, then the cost (lines and
    /// `Cost`s), then the weak preference
    fn rank_cmp(&self, other: &Measure) -> Ordering {
        self.badness
            .cmp(&other.badness)
            .then(self.cost.cmp(&other.cost))
            .then(other.weak_preference.cmp(&self.weak_preference))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Context {
    /// `None` at the start of a line, before its indentation (see `Measure::last`)
    col: Option<u32>,
    indent: u32,
    /// Newlines are not allowed
    flat: bool,
    /// Text must end at most at this column until the first line break (`DocNode::FirstLineWithin`)
    first_line_end: Option<u32>,
}

impl Context {
    /// The column where the next text starts
    fn text_col(&self) -> u32 {
        self.col.unwrap_or(self.indent)
    }

    /// The context after a layout with `measure` placed in this one: at its last column, and free of
    /// the first-line limit after a line break
    fn after(&self, measure: &Measure) -> Context {
        Context {
            col: measure.last,
            first_line_end: self.first_line_end.filter(|_| !measure.multi_line),
            ..*self
        }
    }
}

#[derive(Clone)]
pub(crate) struct Layout {
    measure: Measure,
    content: Content,
}

type LayoutVec = SmallVec<[Layout; 1]>;

#[derive(Clone)]
pub(crate) struct LayoutSet {
    layouts: LayoutVec,
}

impl LayoutSet {
    pub fn new(layouts: LayoutVec) -> Self {
        LayoutSet { layouts }
    }

    pub fn empty() -> Self {
        LayoutSet {
            layouts: LayoutVec::new(),
        }
    }

    pub fn single(layout: Layout) -> Self {
        LayoutSet {
            layouts: smallvec![layout],
        }
    }

    pub fn best(&self) -> Option<&Layout> {
        self.layouts
            .iter()
            .min_by(|a, b| a.measure.rank_cmp(&b.measure))
    }

    /*pub fn dump(&self) {
        println!("LayoutSet [{}]:", self.layouts.len());
        for (idx, layout) in self.layouts.iter().enumerate() {
            println!(
                ">> idx={} last={} baddess={} multi_line={}",
                idx,
                layout.measure.last,
                layout.measure.badness,
                layout.measure.multi_line,
            );
        }
    }*/
}

/// The leaves of a content (`Text`, `Newline`, `Mark`) in order, each with its indentation; it walks
/// an explicit stack instead of recursing, a content nests as deeply as its doc (e.g. a long chain)
struct Leaves<'a> {
    content_arena: &'a ContentArena,
    stack: Vec<(Content, u32)>,
}

impl Iterator for Leaves<'_> {
    type Item = (Content, u32);

    fn next(&mut self) -> Option<(Content, u32)> {
        while let Some((content, indent)) = self.stack.pop() {
            match content {
                Content::Concat(c1, c2) => {
                    self.stack.push((self.content_arena.get(c2), indent));
                    self.stack.push((self.content_arena.get(c1), indent));
                }
                Content::Indent(indent, c) => self.stack.push((self.content_arena.get(c), indent)),
                leaf => return Some((leaf, indent)),
            }
        }
        None
    }
}

pub(crate) struct Renderer<'a> {
    resolver: &'a Resolver<'a>,
    out: String,
    fresh: bool,
    paddings: Vec<u32>,
    marks: usize,
}

impl<'a> Renderer<'a> {
    pub fn render(&mut self, content: Content) {
        for (leaf, indent) in self.resolver.leaves(content) {
            match leaf {
                Content::Text(t) => {
                    let Some(s) = self.resolver.text_of(t) else {
                        continue;
                    };
                    // The indentation of a new line is written with its first text
                    if self.fresh {
                        self.fresh = false;
                        for _ in 0..indent {
                            self.out.push(' ');
                        }
                    }
                    self.out.push_str(s);
                }
                Content::Newline => {
                    self.out.push('\n');
                    self.fresh = true;
                }
                Content::Mark(_) => {
                    for _ in 0..self.paddings[self.marks] {
                        self.out.push(' ');
                    }
                    self.marks += 1;
                }
                Content::Concat(..) | Content::Indent(..) => unreachable!(),
            }
        }
    }
}

struct MarkedComment {
    group: AlignGroup,
    line: u32,
    start: u32,
    end: u32,
}

struct MarkCollector<'a> {
    resolver: &'a Resolver<'a>,
    col: Option<u32>,
    line: u32,
    marks: Vec<MarkedComment>,
    open: Vec<usize>,
}

impl MarkCollector<'_> {
    fn collect(&mut self, content: Content) {
        for (leaf, indent) in self.resolver.leaves(content) {
            match leaf {
                Content::Text(t) => {
                    let Some(s) = self.resolver.text_of(t) else {
                        continue;
                    };
                    let start = self.col.unwrap_or(indent);
                    let mut lines = s.split('\n');
                    let mut col = start + text_width(lines.next().unwrap());
                    for line in lines {
                        self.end_line(col);
                        col = text_width(line);
                    }
                    self.col = Some(col);
                }
                Content::Newline => {
                    self.end_line(self.col.unwrap_or(0));
                    self.col = None;
                }
                Content::Mark(group) => {
                    self.open.push(self.marks.len());
                    self.marks.push(MarkedComment {
                        group,
                        line: self.line,
                        start: self.col.unwrap_or(indent),
                        end: 0,
                    });
                }
                Content::Concat(..) | Content::Indent(..) => unreachable!(),
            }
        }
    }

    fn end_line(&mut self, end: u32) {
        for i in self.open.drain(..) {
            self.marks[i].end = end;
        }
        self.line += 1;
    }
}

/// The padding of each mark, aligning the comments of each run of consecutive marked lines
fn align_marks(resolver: &Resolver, content: Content) -> Vec<u32> {
    let mut collector = MarkCollector {
        resolver,
        col: None,
        line: 0,
        marks: Vec::new(),
        open: Vec::new(),
    };
    collector.collect(content);
    collector.end_line(collector.col.unwrap_or(0));
    let marks = collector.marks;
    let config = resolver.config;
    let mut paddings = vec![0; marks.len()];
    let mut groups: HashMap<AlignGroup, Vec<Vec<usize>>> = HashMap::new();
    for (i, mark) in marks.iter().enumerate() {
        let runs = groups.entry(mark.group).or_default();
        match runs.last_mut() {
            Some(run) if marks[*run.last().unwrap()].line + 1 == mark.line => run.push(i),
            _ => runs.push(vec![i]),
        }
    }
    for mut members in groups.into_values().flatten() {
        members.retain(|&i| marks[i].end <= config.max_width);
        members.sort_by_key(|&i| marks[i].start);
        let n = members.len();
        if n < 2 {
            continue;
        }
        // At most a quarter of the run (at least one line) is left out, keeping two lines
        let few = (n / 4).max(1).min(n - 2);
        let gap = |left_out: usize| {
            marks[members[n - left_out]].start - marks[members[n - left_out - 1]].start
        };
        let outliers = (1..=few)
            .find(|&left_out| gap(left_out) > config.trailing_comment_align_threshold)
            .unwrap_or(0);
        let mut aligned: Option<(Vec<usize>, u32)> = None;
        for left_out in outliers..=few {
            let set = &members[..n - left_out];
            let column = marks[*set.last().unwrap()].start;
            let fits = |&&i: &&usize| marks[i].end + (column - marks[i].start) <= config.max_width;
            let set: Vec<usize> = set.iter().filter(fits).copied().collect();
            if n - set.len() <= few
                && aligned
                    .as_ref()
                    .is_none_or(|(best, _)| set.len() > best.len())
            {
                aligned = Some((set, column));
            }
        }
        if let Some((set, column)) = aligned.filter(|(set, _)| set.len() >= 2) {
            for i in set {
                paddings[i] = column - marks[i].start;
            }
        }
    }
    paddings
}

pub(crate) struct Resolver<'a> {
    doc_arena: DocArena,
    content_arena: ContentArena,
    config: &'a Config,
    root: DocId,
    /// The layouts of the docs that are shared by more alternatives (`Alt` nodes), by context; the
    /// doc is a DAG, so without it a shared doc is resolved again in every alternative containing it
    memo: HashMap<(DocId, Context), LayoutSet>,
}

impl<'a> Resolver<'a> {
    pub fn new(doc_arena: DocArena, config: &'a Config, root: DocId) -> Self {
        Resolver {
            doc_arena,
            config,
            root,
            content_arena: ContentArena::new(),
            memo: HashMap::new(),
        }
    }

    fn leaves(&self, content: Content) -> Leaves<'_> {
        Leaves {
            content_arena: &self.content_arena,
            stack: vec![(content, 0)],
        }
    }

    /// The text of a `Content::Text` leaf; `None` when it is empty
    fn text_of(&self, doc: DocId) -> Option<&str> {
        let s = match self.doc_arena.get_node(doc) {
            DocNode::Token(token) => token.text(),
            DocNode::Text(t) => self.doc_arena.get_text(*t),
            DocNode::Empty => return None,
            _ => unreachable!(),
        };
        (!s.is_empty()).then_some(s)
    }

    pub fn render(&mut self, reserve_size: usize) -> String {
        let layout_set = self.resolve();
        //layout_set.dump();
        let best = layout_set.best().unwrap();
        let mut renderer = Renderer {
            resolver: self,
            out: String::with_capacity(reserve_size),
            fresh: true,
            paddings: align_marks(self, best.content),
            marks: 0,
        };
        renderer.render(best.content);
        renderer.out
    }

    /// The layouts of the root doc at the start of a line; it runs on an explicit stack of `Task`s
    /// instead of recursion, a doc nests deeply (e.g. a long chain of `Concat`s) and the recursion
    /// would overflow the stack
    fn resolve(&mut self) -> LayoutSet {
        let mut tasks = Vec::with_capacity(128);
        // The doc resolved next: a fast path that avoids pushing and popping a `Task::Resolve`,
        // which defers a doc until the tasks above it are done
        let mut next = Some((
            Context {
                col: None,
                indent: 0,
                flat: false,
                first_line_end: None,
            },
            self.root,
        ));
        let mut results: Vec<LayoutSet> = Vec::with_capacity(128);
        loop {
            if let Some((ctx, doc)) = next {
                next = self.resolve_node(ctx, doc, &mut tasks, &mut results);
                continue;
            }
            let Some(task) = tasks.pop() else {
                break;
            };
            match task {
                Task::Resolve(ctx, doc) => next = Some((ctx, doc)),
                Task::Indent(indent) => {
                    let layout_set = results.pop().unwrap();
                    results.push(indented(&mut self.content_arena, indent, layout_set));
                }
                Task::Cost(cost) => {
                    for layout in &mut results.last_mut().unwrap().layouts {
                        let measure = &mut layout.measure;
                        measure.cost += i64::from(cost);
                    }
                }
                Task::WeakPreference(preference) => {
                    for layout in &mut results.last_mut().unwrap().layouts {
                        let measure = &mut layout.measure;
                        measure.weak_preference =
                            measure.weak_preference.saturating_add(preference);
                    }
                }
                Task::KeepTogether(cost) => {
                    for layout in &mut results.last_mut().unwrap().layouts {
                        let measure = &mut layout.measure;
                        if measure.multi_line {
                            measure.cost += i64::from(cost);
                        }
                    }
                }
                Task::MultiLine => {
                    results
                        .last_mut()
                        .unwrap()
                        .layouts
                        .retain(|layout| layout.measure.multi_line);
                }
                Task::IgnoreOverflow => {
                    for layout in &mut results.last_mut().unwrap().layouts {
                        layout.measure.badness = 0;
                    }
                }
                Task::LastLineEndsWithin(max) => {
                    results
                        .last_mut()
                        .unwrap()
                        .layouts
                        .retain(|layout| layout.measure.last.is_none_or(|last| last <= max));
                }
                Task::Fallback(ctx, fallback) => {
                    let primary = results.last().unwrap();
                    if !primary
                        .layouts
                        .iter()
                        .any(|layout| layout.measure.badness == 0)
                    {
                        results.pop();
                        next = Some((ctx, fallback));
                    }
                }
                Task::Alt(ctx, doc) => {
                    let lb = results.pop().unwrap();
                    let la = results.pop().unwrap();
                    let layout_set = merge_layout_sets(la, lb);
                    self.memo.insert((doc, ctx), layout_set.clone());
                    results.push(layout_set);
                }
                Task::ConcatStart(ctx, doc2) => {
                    // The layouts of the first doc stay on the results, under those of the `Concat`
                    // so far; with none, they are the empty result
                    let Some(layout1) = results.last().unwrap().layouts.first() else {
                        continue;
                    };
                    let ctx2 = ctx.after(&layout1.measure);
                    results.push(LayoutSet::empty());
                    tasks.push(Task::Concat(ctx, doc2, 0));
                    next = Some((ctx2, doc2));
                }
                Task::Concat(ctx, doc2, i) => {
                    let mut sub = results.pop().unwrap();
                    let [la, out] = results.last_chunk_mut::<2>().unwrap();
                    let layout1 = &la.layouts[i as usize];
                    if !sub.layouts.is_empty() {
                        // One arena borrow and one node for `layout1`, shared by all the concatenations
                        let c1 = self.content_arena.make(layout1.content);
                        for layout2 in &mut sub.layouts {
                            layout2.measure.badness = layout2
                                .measure
                                .badness
                                .saturating_add(layout1.measure.badness);
                            layout2.measure.cost += layout1.measure.cost;
                            layout2.measure.multi_line |= layout1.measure.multi_line;
                            layout2.measure.weak_preference = layout2
                                .measure
                                .weak_preference
                                .saturating_add(layout1.measure.weak_preference);
                            let c2 = self.content_arena.make(layout2.content);
                            layout2.content = Content::Concat(c1, c2);
                        }
                    }
                    *out = merge_layout_sets(std::mem::replace(out, LayoutSet::empty()), sub);
                    match la.layouts.get(i as usize + 1) {
                        Some(layout1) => {
                            let ctx2 = ctx.after(&layout1.measure);
                            tasks.push(Task::Concat(ctx, doc2, i + 1));
                            next = Some((ctx2, doc2));
                        }
                        None => {
                            let out = results.pop().unwrap();
                            *results.last_mut().unwrap() = out;
                        }
                    }
                }
            }
        }
        debug_assert_eq!(results.len(), 1);
        results.pop().unwrap()
    }

    fn resolve_node(
        &self,
        ctx: Context,
        doc: DocId,
        tasks: &mut Vec<Task>,
        results: &mut Vec<LayoutSet>,
    ) -> Option<(Context, DocId)> {
        let resolve_text = |s: &str| {
            // Before the first line break of a `FirstLineWithin`, the text (its first line) must end
            // within the limit; a single line within it (also `Flat`) contains no line break at all,
            // not even inside a token (a multi-line string literal)
            if let Some(end) = ctx.first_line_end {
                if ctx.flat && s.contains('\n') {
                    return LayoutSet::empty();
                }
                let first_line = s.split('\n').next().unwrap_or("");
                let width = text_width(first_line);
                if ctx.text_col() + width > end {
                    return LayoutSet::empty();
                }
            }
            let measure = if s.is_empty() {
                Measure::empty(&ctx)
            } else {
                // A line break inside a token is not a layout line break, so `ctx.flat` allows it
                Measure::from_text(ctx.text_col(), self.config, s)
            };
            LayoutSet::single(Layout {
                measure,
                content: Content::Text(doc),
            })
        };
        let layout_set = match self.doc_arena.get_node(doc) {
            DocNode::Token(token) => resolve_text(token.text()),
            DocNode::Text(text) => resolve_text(self.doc_arena.get_text(*text)),
            DocNode::Empty => resolve_text(""),
            DocNode::AlignMark(group) => LayoutSet::single(Layout {
                measure: Measure::empty(&ctx),
                content: Content::Mark(*group),
            }),
            DocNode::Newline if ctx.flat => LayoutSet::empty(),
            DocNode::Newline => LayoutSet::single(Layout {
                measure: Measure {
                    last: None,
                    badness: 0,
                    cost: i64::from(LINE_COST),
                    multi_line: true,
                    weak_preference: WeakPreference::NEUTRAL,
                },
                content: Content::Newline,
            }),
            DocNode::Nest(d) => {
                let indent = ctx.indent + self.config.tab_spaces;
                tasks.push(Task::Indent(indent));
                return Some((Context { indent, ..ctx }, *d));
            }
            DocNode::Concat(doc1, doc2) => {
                tasks.push(Task::ConcatStart(ctx, *doc2));
                return Some((ctx, *doc1));
            }
            DocNode::Flat(d) => {
                return Some((Context { flat: true, ..ctx }, *d));
            }
            DocNode::Cost(d, cost) => {
                tasks.push(Task::Cost(*cost));
                return Some((ctx, *d));
            }
            DocNode::WeakPreference(d, preference) => {
                tasks.push(Task::WeakPreference(*preference));
                return Some((ctx, *d));
            }
            DocNode::KeepTogether(d, cost) => {
                tasks.push(Task::KeepTogether(*cost));
                return Some((ctx, *d));
            }
            DocNode::MultiLine(d) => {
                tasks.push(Task::MultiLine);
                return Some((ctx, *d));
            }
            DocNode::ShortLastLine(d) => {
                tasks.push(Task::LastLineEndsWithin(
                    ctx.indent + self.config.tab_spaces,
                ));
                return Some((ctx, *d));
            }
            DocNode::RoomAfter(d, room) => {
                tasks.push(Task::LastLineEndsWithin(
                    self.config.max_width.saturating_sub(*room),
                ));
                return Some((ctx, *d));
            }
            DocNode::FirstLineWithin(d, width) => {
                let end = ctx.text_col().saturating_add(*width);
                let first_line_end = Some(ctx.first_line_end.map_or(end, |e| e.min(end)));
                return Some((
                    Context {
                        first_line_end,
                        ..ctx
                    },
                    *d,
                ));
            }
            DocNode::IgnoreOverflow(d) => {
                tasks.push(Task::IgnoreOverflow);
                return Some((ctx, *d));
            }
            DocNode::Alt(doc1, doc2) => {
                if let Some(layout_set) = self.memo.get(&(doc, ctx)) {
                    results.push(layout_set.clone());
                    return None;
                }
                // `doc1` is resolved first: it is on top
                tasks.push(Task::Alt(ctx, doc));
                tasks.push(Task::Resolve(ctx, *doc2));
                return Some((ctx, *doc1));
            }
            DocNode::Fallback(primary, fallback) => {
                tasks.push(Task::Fallback(ctx, *fallback));
                return Some((ctx, *primary));
            }
            DocNode::Align(d) => {
                let indent = ctx.text_col();
                tasks.push(Task::Indent(indent));
                return Some((Context { indent, ..ctx }, *d));
            }
        };
        results.push(layout_set);
        None
    }
}

enum Task {
    Resolve(Context, DocId),
    Indent(u32),
    Cost(Cost),
    WeakPreference(WeakPreference),
    KeepTogether(Cost),
    MultiLine,
    /// Only the layouts whose last line ends at most at the column (`DocNode::ShortLastLine`,
    /// `DocNode::RoomAfter`)
    LastLineEndsWithin(u32),
    IgnoreOverflow,
    Alt(Context, DocId),
    Fallback(Context, DocId),
    ConcatStart(Context, DocId),
    Concat(Context, DocId, u32),
}

/// The layouts rendered with `indent`; a layout ending with a line break keeps `last: None`, the next
/// text after it is indented by the indentation current there
fn indented(content_arena: &mut ContentArena, indent: u32, mut layout_set: LayoutSet) -> LayoutSet {
    for layout in &mut layout_set.layouts {
        layout.content = Content::Indent(indent, content_arena.make(layout.content));
    }
    layout_set
}

/// The Pareto frontier of the layouts by (`last`, rank); a layout ending with a line break (`last: None`)
/// counts as ending leftmost, it leaves the most room for what follows
fn merge_layout_sets(a: LayoutSet, b: LayoutSet) -> LayoutSet {
    let mut out = LayoutVec::new();
    let mut a_it = a.layouts.into_iter();
    let mut la = a_it.next();
    let mut b_it = b.layouts.into_iter();
    let mut lb = b_it.next();
    loop {
        let choose_a = match (&la, &lb) {
            (Some(x), Some(y)) => x.measure.last <= y.measure.last,
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => break,
        };

        let m;
        if choose_a {
            m = la.unwrap();
            la = a_it.next();
        } else {
            m = lb.unwrap();
            lb = b_it.next();
        }

        match out.last() {
            Some(prev) if prev.measure.last == m.measure.last => {
                if m.measure.rank_le(&prev.measure) {
                    out.pop();
                    out.push(m);
                }
            }
            Some(prev) if prev.measure.rank_le(&m.measure) => {}
            _ => out.push(m),
        }
    }
    LayoutSet::new(out)
}
