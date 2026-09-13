use crate::config::Config;
use crate::measure::Measure;
use ra_ap_syntax::{SyntaxNode, SyntaxToken, TextRange};

#[derive(Clone, Debug)]
struct Content(String);

#[derive(Clone, Debug)]
struct Layout {
    measure: Measure,
    content: Content,
}

impl Content {
    pub fn new() -> Self {
        Content(String::new())
    }

    pub fn render(&self) -> &str {
        self.0.as_str()
    }

    fn concat(&self, measure: &Measure, other: &Content) -> Content {
        let mut result = self.0.clone();
        let indent = measure.last_width();
        let mut it = other.0.split('\n');
        result.push_str(&it.next().unwrap());
        for line in it {
            result.push('\n');
            for _ in 0..indent {
                result.push(' ');
            }
            result.push_str(&line);
        }
        Content(result)
    }

    pub fn add_char(&mut self, c: char) {
        self.0.push(c);
    }

    pub fn add_text(&mut self, text: &str) {
        self.0.push_str(text);
    }
}

#[derive(Debug)]
pub(crate) struct Doc {
    layouts: Vec<Layout>,
}

impl Doc {
    pub fn new() -> Self {
        Doc {
            layouts: vec![Layout {
                measure: Measure::text(""),
                content: Content::new(),
            }],
        }
    }

    pub fn add_text(&mut self, max_width: u32, text: &str) {
        self.layouts.retain_mut(|layout| {
            layout.measure.expand_last(text.len() as u32);
            if layout.measure.is_valid(max_width) {
                layout.content.add_text(text);
                true
            } else {
                false
            }
        })
    }

    pub fn add_char(&mut self, max_width: u32, char: char) {
        self.layouts.retain_mut(|layout| {
            layout.measure.expand_last(1);
            if layout.measure.is_valid(max_width) {
                layout.content.add_char(char);
                true
            } else {
                false
            }
        })
    }

    pub fn add_ident(&mut self, ident: u32) {
        if ident == 0 {
            return;
        }
        for layout in &mut self.layouts {
            layout.measure.expand_last(ident);
            for _ in 0..ident {
                layout.content.add_char(' ');
            }
        }
    }

    pub fn new_line(&mut self) {
        for layout in &mut self.layouts {
            layout.measure.new_line();
            layout.content.add_char('\n');
        }
    }

    pub fn text(cfg: &Config, s: String) -> Doc {
        let m = Measure::text(&s);
        Doc {
            layouts: if m.is_valid(cfg.max_width) {
                vec![Layout {
                    measure: m,
                    content: Content(s),
                }]
            } else {
                Vec::new()
            },
        }
    }

    pub fn concat(&self, cfg: &Config, other: &Doc) -> Doc {
        let mut candidates = Vec::new();
        for l1 in &self.layouts {
            for l2 in &other.layouts {
                let m = l1.measure.concat(&l2.measure);
                dbg!(&m, m.is_valid(cfg.max_width));
                if m.is_valid(cfg.max_width) {
                    candidates.push(Layout {
                        measure: m,
                        content: l1.content.concat(&l1.measure, &l2.content),
                    });
                }
            }
        }
        let mut layouts = Vec::new();
        pareto_filter(&mut layouts, candidates);
        Doc { layouts }
    }

    pub fn union(&mut self, docs: Doc) {
        pareto_filter(&mut self.layouts, docs.layouts.into_iter());
    }

    pub fn render(&self) -> Option<&str> {
        let Some(layout) = self.layouts.iter().min_by_key(|layout| &layout.measure) else {
            return None;
        };
        Some(layout.content.render())
    }
}

fn pareto_filter(frontier: &mut Vec<Layout>, items: impl IntoIterator<Item = Layout>) {
    for layout in items {
        if frontier
            .iter()
            .any(|l| l.measure.dominates(&layout.measure))
        {
            continue;
        }
        frontier.retain(|l| !layout.measure.dominates(&l.measure));
        frontier.push(layout);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct DocId(u32);

impl DocId {
    pub fn is_empty(&self) -> bool {
        *self == EMPTY_DOC
    }
}

// #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
// pub(crate) struct StrId(u32);

// NEW

pub(crate) enum DocNode {
    Token(SyntaxToken),
    Text(String),
    Newline,
    Concat(DocId, DocId),
    Alt(DocId, DocId),
    Nest(DocId),
    Align(DocId),
}

pub(crate) struct DocArena {
    nodes: Vec<DocNode>,
}


pub(crate) const NEWLINE_DOC: DocId = DocId(0);
pub(crate) const EMPTY_DOC: DocId = DocId(1);
pub(crate) const SPACE_DOC: DocId = DocId(2);
pub(crate) const NEWLINE_OR_SPACE_DOC: DocId = DocId(3);
pub(crate) const EMPTYLINE_DOC: DocId = DocId(4);
pub(crate) const COMMA_DOC: DocId = DocId(5);
pub(crate) const SEMICOLON_DOC: DocId = DocId(6);


impl DocArena {
    pub fn new() -> Self {
        DocArena {
            nodes: vec![
                DocNode::Newline,
                DocNode::Text(String::new().into()),
                DocNode::Text(" ".into()),
                DocNode::Alt(NEWLINE_DOC, SPACE_DOC),
                DocNode::Concat(NEWLINE_DOC, NEWLINE_DOC),
                DocNode::Text(",".into()),
                DocNode::Text(";".into()),
            ],
        }
    }

    #[inline]
    pub fn make(&mut self, kind: DocNode) -> DocId {
        let id = DocId(self.nodes.len() as u32);
        self.nodes.push(kind);
        id
    }

    pub fn dump(&self, root: DocId) {
        self.dump_helper(root, 0);
    }

    pub fn get_node(&self, doc: DocId) -> &DocNode {
        &self.nodes[doc.0 as usize]
    }

    fn dump_helper(&self, doc: DocId, indent: usize) {
        let make_ident = || {
            for _ in 0..indent {
                print!("  ");
            }
        };
        make_ident();
        print!("#{} ", doc.0);
        match self.get_node(doc) {
            DocNode::Token(token) => println!("Token({:?})", token.text()),
            DocNode::Text(text) => println!("Text({:?})", text),
            DocNode::Newline => println!("Newline"),
            DocNode::Nest(d) => {
                println!("Nest(");
                self.dump_helper(*d, indent + 1);
                make_ident();
                println!(")");
            }
            DocNode::Concat(d1, d2) => {
                println!("Concat(");
                self.dump_helper(*d1, indent + 1);
                self.dump_helper(*d2, indent + 1);
                make_ident();
                println!(")");
            }
            DocNode::Alt(d1, d2) => {
                println!("Alt(");
                self.dump_helper(*d1, indent + 1);
                self.dump_helper(*d2, indent + 1);
                make_ident();
                println!(")");
            }
            DocNode::Align(d) => {
                println!("Align(");
                self.dump_helper(*d, indent + 1);
                make_ident();
                println!(")");
            }
        }
    }
}
