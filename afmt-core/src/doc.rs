use std::collections::HashSet;
use std::fmt::Write;

use ra_ap_syntax::SyntaxToken;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct DocId(u32);

impl DocId {
    pub fn is_empty(&self) -> bool {
        *self == EMPTY_DOC
    }
}

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub(crate) struct Cost(u32);

impl Cost {
    #[inline]
    pub fn new(value: u32) -> Self {
        Cost(value)
    }

    #[inline]
    pub fn saturating_add(self, other: Cost) -> Self {
        Cost::new(self.0.saturating_add(other.0))
    }
}

pub(crate) enum DocNode {
    Token(SyntaxToken),
    Text(String),
    Newline,
    Concat(DocId, DocId),
    Alt(DocId, DocId),
    Nest(DocId),
    Align(DocId),
    /// Only layouts of the inner doc that fit on a single line
    Flat(DocId),
    /// Only layouts of the inner doc that span more than one line
    MultiLine(DocId),
    /// The inner doc should stay on one line if possible
    /// If broken, it adds into costs
    KeepTogether(DocId, Cost),
    /// Every layout of the inner doc costs a split: it is used only when the layouts without it are
    /// worse, e.g. a layout the Style Guide accepts only as a fallback
    Dispreferred(DocId, Cost),
    /// Only layouts of the inner doc whose last line ends at most at the indentation of a nested line
    /// (`indent + indent_width`), e.g. a short chain root that the next element may follow: `x.baz?`
    ShortLastLine(DocId),
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
                DocNode::Text(String::new()),
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

    /// The doc graph reachable from `root` in the Graphviz DOT format; shared nodes are written once
    pub fn to_dot(&self, root: DocId) -> String {
        let mut out = String::from(
            "digraph doc {\n    ordering=out;\n    node [style=filled, fontname=\"monospace\"];\n",
        );
        let mut visited = HashSet::new();
        let mut stack = vec![root];
        while let Some(doc) = stack.pop() {
            if !visited.insert(doc) {
                continue;
            }
            let node = self.get_node(doc);
            let (label, shape, color) = match node {
                DocNode::Token(token) => (format!("'{}'", token.text()), "box", "lightblue"),
                DocNode::Text(text) => (format!("'{}'", text), "box", "lightyellow"),
                DocNode::Newline => ("Newline".into(), "rect", "gray"),
                DocNode::Concat(..) => ("Concat".into(), "ellipse", "white"),
                DocNode::Alt(..) => ("Alt".into(), "diamond", "orange"),
                DocNode::Nest(_) => ("Nest".into(), "trapezium", "palegreen"),
                DocNode::Align(_) => ("Align".into(), "parallelogram", "aquamarine"),
                DocNode::Flat(_) => ("Flat".into(), "hexagon", "pink"),
                DocNode::MultiLine(_) => ("MultiLine".into(), "octagon", "plum"),
                DocNode::KeepTogether(_, cost) => (format!("KeepTogether {}", cost.0), "doubleoctagon", "khaki"),
                DocNode::Dispreferred(_, cost) => (format!("Dispreferred {}", cost.0), "tripleoctagon", "khaki"),
                DocNode::ShortLastLine(_) => ("ShortLastLine".into(), "house", "lightpink"),
            };
            let label = label
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\l");
            writeln!(
                out,
                "    n{0} [label=\"#{0}\\n{1}\", shape={2}, fillcolor={3}];",
                doc.0, label, shape, color
            )
            .unwrap();
            let children: Vec<(DocId, &str)> = match node {
                DocNode::Token(_) | DocNode::Text(_) | DocNode::Newline => vec![],
                DocNode::Concat(d1, d2) => vec![(*d1, "1"), (*d2, "2")],
                DocNode::Alt(d1, d2) => vec![(*d1, ""), (*d2, "")],
                DocNode::Nest(d)
                | DocNode::Align(d)
                | DocNode::Flat(d)
                | DocNode::MultiLine(d)
                | DocNode::KeepTogether(d, _)
                | DocNode::Dispreferred(d, _)
                | DocNode::ShortLastLine(d) => vec![(*d, "")],
            };
            for (child, edge_label) in &children {
                write!(out, "    n{} -> n{}", doc.0, child.0).unwrap();
                if !edge_label.is_empty() {
                    write!(out, " [label=\"{}\"]", edge_label).unwrap();
                }
                out.push_str(";\n");
            }
            // Reversed, so the first child is written first
            stack.extend(children.iter().rev().map(|(child, _)| *child));
        }
        out.push_str("}\n");
        out
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
            DocNode::Flat(d) => {
                println!("Flat(");
                self.dump_helper(*d, indent + 1);
                make_ident();
                println!(")");
            }
            DocNode::Dispreferred(d, cost) => {
                println!("Dispreferred[{}](", cost.0);
                self.dump_helper(*d, indent + 1);
                make_ident();
                println!(")");
            }
            DocNode::KeepTogether(d, cost) => {
                println!("KeepTogether[{}](", cost.0);
                self.dump_helper(*d, indent + 1);
                make_ident();
                println!(")");
            }
            DocNode::ShortLastLine(d) => {
                println!("ShortLastLine(");
                self.dump_helper(*d, indent + 1);
                make_ident();
                println!(")");
            }
            DocNode::MultiLine(d) => {
                println!("MultiLine(");
                self.dump_helper(*d, indent + 1);
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
