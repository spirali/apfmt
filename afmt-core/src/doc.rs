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

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub(crate) struct AlignGroup(u32);

impl AlignGroup {
    #[inline]
    pub fn new(value: u32) -> Self {
        AlignGroup(value)
    }

    pub fn bump(&mut self) -> AlignGroup {
        let out = AlignGroup(self.0);
        self.0 += 1;
        out
    }
}

pub(crate) enum DocNode {
    Empty,
    Token(SyntaxToken),
    // TODO: Move into a container that is just usize length (ThinBox on nightly?)
    Text(Box<str>),
    Newline,
    Concat(DocId, DocId),
    Alt(DocId, DocId, AltFlatInfo),
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
    /// Padded by the renderer to align the trailing comments of a group
    AlignMark(AlignGroup),
}

// On 64-bit targets, the payload of `Alt` with its `AltFlatInfo` fits in the space of the largest
// variant, `Text(Box<str>)`, so the cache makes no node larger; on 32-bit targets, where a
// `Box<str>` is 8 bytes, it does
#[cfg(target_pointer_width = "64")]
const _: () =
    assert!(std::mem::size_of::<(DocId, DocId, AltFlatInfo)>() <= std::mem::size_of::<Box<str>>());

/// What the single-line walks of the formatter (`Formatter::flat_width`, `Formatter::has_flat_layout`)
/// need to know about an `Alt` node, computed when it is created: a doc shared by many alternatives is
/// then walked once, not once per path to it (exponential in the nesting)
#[derive(Clone, Copy, Debug)]
pub(crate) struct AltFlatInfo {
    /// The width of the narrowest single-line branch, `u32::MAX` if no branch has one
    width: u32,
    /// Whether a branch may have a layout under `Flat`
    has_flat_layout: bool,
}

impl AltFlatInfo {
    pub fn new(width: Option<usize>, has_flat_layout: bool) -> Self {
        // A width that does not fit in `u32` (never in practice) is saturated, not lost
        let width = width.map_or(u32::MAX, |w| u32::try_from(w).unwrap_or(u32::MAX - 1));
        AltFlatInfo {
            width,
            has_flat_layout,
        }
    }

    pub fn width(self) -> Option<usize> {
        (self.width != u32::MAX).then_some(self.width as usize)
    }

    pub fn has_flat_layout(self) -> bool {
        self.has_flat_layout
    }
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
                DocNode::Empty,
                DocNode::Text(" ".into()),
                DocNode::Alt(NEWLINE_DOC, SPACE_DOC, AltFlatInfo::new(Some(1), true)),
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
                DocNode::KeepTogether(_, cost) => {
                    (format!("KeepTogether {}", cost.0), "doubleoctagon", "khaki")
                }
                DocNode::Dispreferred(_, cost) => {
                    (format!("Dispreferred {}", cost.0), "tripleoctagon", "khaki")
                }
                DocNode::ShortLastLine(_) => ("ShortLastLine".into(), "house", "lightpink"),
                DocNode::Empty => ("<empty>".into(), "box", "lightyellow"),
                DocNode::AlignMark(group) => {
                    (format!("AlignMark {}", group.0), "box", "lightyellow")
                }
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
                DocNode::Token(_)
                | DocNode::Text(_)
                | DocNode::Newline
                | DocNode::Empty
                | DocNode::AlignMark(_) => vec![],
                DocNode::Concat(d1, d2) => vec![(*d1, "1"), (*d2, "2")],
                DocNode::Alt(d1, d2, _) => vec![(*d1, ""), (*d2, "")],
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
            DocNode::Empty => println!("Empty"),
            DocNode::AlignMark(group) => println!("AlignMark({})", group.0),
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
            DocNode::Alt(d1, d2, _) => {
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
