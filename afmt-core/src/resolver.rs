use crate::config::Config;
use crate::doc::{DocArena, DocId, DocNode, EMPTY_DOC};
use smallvec::{SmallVec, smallvec};
use std::cmp::Ordering;
use std::rc::Rc;

#[derive(Clone)]
struct Measure {
    last: u32,
    badness: u32,
    height: u32,
    is_full: bool,
}

impl Measure {
    fn from_text(col: u32, page_width: u32, len: u32) -> Self {
        let last = col + len;
        let badness = if last > page_width {
            let m = page_width.max(col);
            let a = m - page_width;
            let b = last - m;
            b * (2 * a + b)
        } else {
            0
        };
        Measure {
            last,
            badness,
            height: 0,
            is_full: false,
        }
    }

    fn cost_le(&self, other: &Measure) -> bool {
        match self.cost_cmp(other) {
            Ordering::Less | Ordering::Equal => true,
            Ordering::Greater => false,
        }
    }

    fn cost_cmp(&self, other: &Measure) -> Ordering {
        match self.badness.cmp(&other.badness) {
            Ordering::Equal => self.height.cmp(&other.height),
            other => other,
        }
    }
}

enum AltChoice {
    Left,
    Right,
}

impl AltChoice {
    pub fn apply<T>(&self, left: T, right: T) -> T {
        match self {
            AltChoice::Left => left,
            AltChoice::Right => right,
        }
    }
}

#[derive(Clone)]
pub enum Content {
    Text(DocId),
    Newline,
    Concat(Rc<Content>, Rc<Content>),
    Indent(u32, Rc<Content>),
}

#[derive(Clone, Copy)]
pub struct Context {
    col: u32,
    indent: u32,
    no_text: bool,
}

pub(crate) struct Layout {
    measure: Measure,
    content: Rc<Content>,
}

type LayoutVec = SmallVec<[Layout; 1]>;

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

    pub fn dump(&self) {
        println!("LayoutSet [{}]:", self.layouts.len());
        for (idx, layout) in self.layouts.iter().enumerate() {
            println!(
                ">> idx={} last={} baddess={} height={} is_full={}",
                idx,
                layout.measure.last,
                layout.measure.badness,
                layout.measure.height,
                layout.measure.is_full
            );
        }
    }
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
                if self.fresh {
                    self.fresh = false;
                    for _ in 0..self.indent {
                        self.out.push(' ');
                    }
                }
                let s = match self.resolver.doc_arena.get_node(*t) {
                    DocNode::Token(token) => {
                        token.text()
                    }
                    DocNode::Text(t) => {
                        t.as_str()
                    }
                    _ => unreachable!(),
                };
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
                self.render(&c1);
                self.render(&c2);
            }
        }
    }
}

pub(crate) struct Resolver<'a> {
    doc_arena: DocArena,
    config: &'a Config,
    root: DocId,
    new_line: Rc<Content>,
}

impl<'a> Resolver<'a> {
    pub fn new(doc_arena: DocArena, config: &'a Config, root: DocId) -> Self {
        Resolver {
            doc_arena,
            config,
            root,
            new_line: Rc::new(Content::Newline),
        }
    }

    pub fn render(&self, reserve_size: usize) -> String {
        let layout_set = self.resolve(
            Context {
                col: 0,
                indent: 0,
                no_text: false,
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
            if ctx.no_text && !s.is_empty() {
                LayoutSet::empty()
            } else {
                LayoutSet::single(Layout {
                    measure: Measure::from_text(ctx.col, self.config.max_width, unicode_width::UnicodeWidthStr::width(s) as u32),
                    content: Rc::new(Content::Text(doc)),
                })
            }
        };
        match self.doc_arena.get_node(doc) {
            DocNode::Token(token) => {
                resolve_text(token.text())
            }
            DocNode::Text(text) => {
                resolve_text(text.as_str())
            }
            DocNode::Newline => LayoutSet::single(Layout {
                measure: Measure {
                    last: ctx.indent,
                    badness: 0,
                    height: 1,
                    is_full: false,
                },
                content: self.new_line.clone(),
            }),
            DocNode::Nest(d) => {
                let indent = ctx.indent + self.config.indent_width;
                let mut layout_set = self.resolve(
                    Context {
                        indent,
                        ..ctx
                    },
                    *d,
                );
                for layout in &mut layout_set.layouts {
                    layout.content = Rc::new(Content::Indent(indent, layout.content.clone()));
                }
                layout_set
            }
            DocNode::Concat(doc1, doc2) => {
                let la = self.resolve(ctx, *doc1);
                let mut out = LayoutSet::empty();
                for layout1 in la.layouts {
                    let mut sub = self.resolve(
                        Context {
                            col: layout1.measure.last,
                            indent: ctx.indent,
                            no_text: ctx.no_text,
                        },
                        *doc2,
                    );
                    for layout2 in &mut sub.layouts {
                        layout2.measure.badness += layout1.measure.badness;
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
            DocNode::Alt(doc1, doc2) => {
                let la = self.resolve(ctx, *doc1);
                let lb = self.resolve(ctx, *doc2);
                merge_layout_sets(la, lb)
            }
            DocNode::Align(doc) => {
                let mut layout_set = self.resolve(
                    Context {
                        indent: ctx.col,
                        ..ctx
                    },
                    *doc,
                );
                for layout in &mut layout_set.layouts {
                    layout.content = Rc::new(Content::Indent(ctx.col, layout.content.clone()));
                }
                layout_set
            }
        }
    }
}

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
