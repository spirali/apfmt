use crate::config::Config;
use crate::doc::{Cost, DocArena, DocId, DocNode};
use smallvec::{SmallVec, smallvec};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone)]
struct Measure {
    /// The column after the layout; `None` when it ends with a line break: the indentation of the new
    /// line is written with the next text, at the indentation current there (`Renderer`), e.g. the
    /// outer one for `)` after a broken list whose last line break is inside its `Nest`
    last: Option<u32>,
    /// The overflow beyond the page width, quadratic in its length; saturating, a single line can be
    /// very long (e.g. an embedded blob of data)
    badness: u64,
    cost: Cost,
    height: u32,
}

/// The overflow of a line part from `col` to `last` beyond `page_width`, quadratic in its length
fn overflow(col: u32, last: u32, page_width: u32) -> u64 {
    if last <= page_width {
        return 0;
    }
    let m = page_width.max(col);
    let a = u64::from(m - page_width);
    let b = u64::from(last - m);
    b.saturating_mul(a.saturating_mul(2).saturating_add(b))
}

impl Measure {
    /// `text` starting at `col`; a text with line breaks (a multi-line string literal or comment) is
    /// printed as it is, its following lines start at column 0
    fn from_text(col: u32, page_width: u32, text: &str) -> Self {
        let mut badness = 0u64;
        let mut height = 0;
        let mut start = col;
        let mut last = col;
        for (i, line) in text.split('\n').enumerate() {
            if i > 0 {
                height += 1;
                start = 0;
            }
            last = start + unicode_width::UnicodeWidthStr::width(line) as u32;
            badness = badness.saturating_add(overflow(start, last, page_width));
        }
        Measure {
            last: Some(last),
            badness,
            cost: Cost::new(0),
            height,
        }
    }

    /// A layout without text, e.g. an empty text
    fn empty(ctx: &Context) -> Self {
        Measure {
            last: ctx.col,
            badness: 0,
            cost: Cost::new(0),
            height: 0,
        }
    }

    fn cost_le(&self, other: &Measure) -> bool {
        match self.cost_cmp(other) {
            Ordering::Less | Ordering::Equal => true,
            Ordering::Greater => false,
        }
    }

    fn cost_cmp(&self, other: &Measure) -> Ordering {
        self.badness
            .cmp(&other.badness)
            .then(self.cost.cmp(&other.cost))
            .then(self.height.cmp(&other.height))
    }
}

#[derive(Clone)]
pub enum Content {
    Text(DocId),
    Newline,
    Concat(Rc<Content>, Rc<Content>),
    Indent(u32, Rc<Content>),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Context {
    /// `None` at the start of a line, before its indentation (see `Measure::last`)
    col: Option<u32>,
    indent: u32,
    /// Newlines are not allowed
    flat: bool,
}

impl Context {
    /// The column where the next text starts
    fn text_col(&self) -> u32 {
        self.col.unwrap_or(self.indent)
    }
}

#[derive(Clone)]
pub(crate) struct Layout {
    measure: Measure,
    content: Rc<Content>,
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
        //return Some(&self.layouts[0]);
        self.layouts
            .iter()
            .min_by(|a, b| a.measure.cost_cmp(&b.measure))
    }

    /*pub fn dump(&self) {
        println!("LayoutSet [{}]:", self.layouts.len());
        for (idx, layout) in self.layouts.iter().enumerate() {
            println!(
                ">> idx={} last={} baddess={} height={}",
                idx,
                layout.measure.last,
                layout.measure.badness,
                layout.measure.height,
            );
        }
    }*/
}

pub(crate) struct Renderer<'a> {
    resolver: &'a Resolver<'a>,
    out: String,
    indent: u32,
    fresh: bool,
}

impl<'a> Renderer<'a> {
    pub fn render(&mut self, content: &Content) {
        match content {
            Content::Text(t) => {
                let s = match self.resolver.doc_arena.get_node(*t) {
                    DocNode::Token(token) => token.text(),
                    DocNode::Text(t) => t.as_str(),
                    _ => unreachable!(),
                };
                if s.is_empty() {
                    return;
                }
                // The indentation of a new line is written with its first text
                if self.fresh {
                    self.fresh = false;
                    for _ in 0..self.indent {
                        self.out.push(' ');
                    }
                }
                self.out.push_str(s);
            }
            Content::Newline => {
                self.out.push('\n');
                self.fresh = true;
            }
            Content::Indent(indent, d) => {
                let old = self.indent;
                self.indent = *indent;
                self.render(d);
                self.indent = old;
            }
            Content::Concat(c1, c2) => {
                self.render(c1);
                self.render(c2);
            }
        }
    }
}

pub(crate) struct Resolver<'a> {
    doc_arena: DocArena,
    config: &'a Config,
    root: DocId,
    new_line: Rc<Content>,
    /// The layouts of the docs that are shared by more alternatives (`Alt` nodes), by context; the
    /// doc is a DAG, so without it a shared doc is resolved again in every alternative containing it
    memo: RefCell<HashMap<(DocId, Context), LayoutSet>>,
}

impl<'a> Resolver<'a> {
    pub fn new(doc_arena: DocArena, config: &'a Config, root: DocId) -> Self {
        Resolver {
            doc_arena,
            config,
            root,
            new_line: Rc::new(Content::Newline),
            memo: RefCell::new(HashMap::new()),
        }
    }

    pub fn render(&self, reserve_size: usize) -> String {
        let layout_set = self.resolve(
            Context {
                col: None,
                indent: 0,
                flat: false,
            },
            self.root,
        );
        //layout_set.dump();
        let best = layout_set.best().unwrap();
        let mut renderer = Renderer {
            resolver: self,
            out: String::with_capacity(reserve_size),
            indent: 0,
            fresh: true,
        };
        renderer.render(&best.content);
        renderer.out
    }

    pub fn resolve(&self, ctx: Context, doc: DocId) -> LayoutSet {
        let resolve_text = |s: &str| {
            let measure = if s.is_empty() {
                Measure::empty(&ctx)
            } else {
                // A line break inside a token is not a layout line break, so `ctx.flat` allows it
                Measure::from_text(ctx.text_col(), self.config.max_width, s)
            };
            LayoutSet::single(Layout {
                measure,
                content: Rc::new(Content::Text(doc)),
            })
        };
        match self.doc_arena.get_node(doc) {
            DocNode::Token(token) => resolve_text(token.text()),
            DocNode::Text(text) => resolve_text(text.as_str()),
            DocNode::Newline if ctx.flat => LayoutSet::empty(),
            DocNode::Newline => LayoutSet::single(Layout {
                measure: Measure {
                    last: None,
                    badness: 0,
                    cost: Cost::new(0),
                    height: 1,
                },
                content: self.new_line.clone(),
            }),
            DocNode::Nest(d) => {
                let indent = ctx.indent + self.config.indent_width;
                let layout_set = self.resolve(Context { indent, ..ctx }, *d);
                indented(indent, layout_set)
            }
            DocNode::Concat(doc1, doc2) => {
                let la = self.resolve(ctx, *doc1);
                let mut out = LayoutSet::empty();
                for layout1 in la.layouts {
                    let mut sub = self.resolve(
                        Context {
                            col: layout1.measure.last,
                            ..ctx
                        },
                        *doc2,
                    );
                    for layout2 in &mut sub.layouts {
                        layout2.measure.badness = layout2
                            .measure
                            .badness
                            .saturating_add(layout1.measure.badness);
                        layout2.measure.cost = layout2.measure.cost.saturating_add(layout1.measure.cost);
                        layout2.measure.height += layout1.measure.height;
                        layout2.content = Rc::new(Content::Concat(
                            layout1.content.clone(),
                            layout2.content.clone(),
                        ));
                    }
                    out = merge_layout_sets(out, sub);
                }
                out
            }
            DocNode::Flat(doc) => self.resolve(Context { flat: true, ..ctx }, *doc),
            DocNode::Dispreferred(doc, cost) => {
                let mut layout_set = self.resolve(ctx, *doc);
                for layout in &mut layout_set.layouts {
                    layout.measure.cost = layout.measure.cost.saturating_add(*cost);
                }
                layout_set
            }
            DocNode::KeepTogether(doc, cost) => {
                let mut layout_set = self.resolve(ctx, *doc);
                for layout in &mut layout_set.layouts {
                    if layout.measure.height > 0 {
                        layout.measure.cost = layout.measure.cost.saturating_add(*cost);
                    }
                }
                layout_set
            }
            DocNode::MultiLine(doc) => {
                let mut layout_set = self.resolve(ctx, *doc);
                layout_set
                    .layouts
                    .retain(|layout| layout.measure.height > 0);
                layout_set
            }
            DocNode::ShortLastLine(doc) => {
                let max = ctx.indent + self.config.indent_width;
                let mut layout_set = self.resolve(ctx, *doc);
                layout_set
                    .layouts
                    .retain(|layout| layout.measure.last.is_none_or(|last| last <= max));
                layout_set
            }
            DocNode::Alt(doc1, doc2) => {
                if let Some(layout_set) = self.memo.borrow().get(&(doc, ctx)) {
                    return layout_set.clone();
                }
                let la = self.resolve(ctx, *doc1);
                let lb = self.resolve(ctx, *doc2);
                let layout_set = merge_layout_sets(la, lb);
                self.memo
                    .borrow_mut()
                    .insert((doc, ctx), layout_set.clone());
                layout_set
            }
            DocNode::Align(doc) => {
                let indent = ctx.text_col();
                let layout_set = self.resolve(Context { indent, ..ctx }, *doc);
                indented(indent, layout_set)
            }
        }
    }
}

/// The layouts rendered with `indent`; a layout ending with a line break keeps `last: None`, the next
/// text after it is indented by the indentation current there
fn indented(indent: u32, mut layout_set: LayoutSet) -> LayoutSet {
    for layout in &mut layout_set.layouts {
        layout.content = Rc::new(Content::Indent(indent, layout.content.clone()));
    }
    layout_set
}

/// The Pareto frontier of the layouts by (`last`, cost); a layout ending with a line break (`last: None`)
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
                if m.measure.cost_le(&prev.measure) {
                    out.pop();
                    out.push(m);
                }
            }
            Some(prev) if prev.measure.cost_le(&m.measure) => {}
            _ => out.push(m),
        }
    }
    LayoutSet::new(out)
}
