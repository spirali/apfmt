use std::collections::{HashMap, HashSet};
use std::fmt::Write;

use ra_ap_syntax::SyntaxToken;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct DocId(u32);

impl DocId {
    pub fn is_empty(&self) -> bool {
        *self == EMPTY_DOC
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct DocStrId(u32);

/// How much a layout is preferred (`DocNode::Preference`, `DocNode::KeepTogether`): positive is
/// preferred, negative dispreferred, in splits (`DISPREFERRED` is one); compared right after the
/// overflow, before the number of lines
#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub(crate) struct Preference(i32);

impl Preference {
    pub const NEUTRAL: Preference = Preference(0);
    pub const DISPREFERRED: Preference = Preference(-1);

    #[inline]
    pub const fn new(value: i32) -> Self {
        Preference(value)
    }

    #[inline]
    pub fn saturating_add(self, other: Preference) -> Self {
        Preference(self.0.saturating_add(other.0))
    }
}

/// How much a layout is preferred among layouts that are otherwise equal (the same overflow, splits
/// and lines, see `DocNode::WeakPreference`): positive is preferred, negative dispreferred; a type of
/// its own, never mixed with a `Preference`
#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub(crate) struct WeakPreference(i32);

impl WeakPreference {
    pub const NEUTRAL: WeakPreference = WeakPreference(0);
    pub const PREFERRED: WeakPreference = WeakPreference(1);

    #[inline]
    pub fn saturating_add(self, other: WeakPreference) -> Self {
        WeakPreference(self.0.saturating_add(other.0))
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
    Text(DocStrId),
    Newline,
    Concat(DocId, DocId),
    Alt(DocId, DocId),
    Nest(DocId),
    Align(DocId),
    /// Only layouts of the inner doc that fit on a single line
    Flat(DocId),
    /// Only layouts of the inner doc that span more than one line
    MultiLine(DocId),
    /// The inner doc should stay on one line if possible: its layouts spanning more lines get the
    /// preference (negative, e.g. a split)
    KeepTogether(DocId, Preference),
    /// Every layout of the inner doc gets the preference, compared right after the overflow: e.g. a
    /// split, a layout the Style Guide accepts only as a fallback, used only when the layouts without
    /// it are worse
    Preference(DocId, Preference),
    /// Every layout of the inner doc gets the preference, compared only after the number of lines:
    /// it decides between layouts that are otherwise equally good, e.g. the Style Guide's preference
    /// that does not justify a longer layout
    WeakPreference(DocId, WeakPreference),
    /// Only layouts of the inner doc whose last line ends at most at the indentation of a nested line
    /// (`indent + indent_width`), e.g. a short chain root that the next element may follow: `x.baz?`
    ShortLastLine(DocId),
    /// The inner doc does not count as overflow when it extends beyond the page width, e.g. a
    /// comment after code that ends the line: a long comment does not change the layout of the code
    /// in front of it (as in rustfmt)
    IgnoreOverflow(DocId),
    /// Padded by the renderer to align the trailing comments of a group
    AlignMark(AlignGroup),
}

/// What the single-line walks (`DocArena::flat_width`, `DocArena::has_flat_layout`) need to know
/// about an `Alt` node, computed the first time a walk reaches it and kept in `DocArena::alt_infos`: a
/// doc shared by many alternatives is then walked once, not once per path to it (exponential in the
/// nesting)
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
    strings: Vec<Box<str>>,
    /// The single-line facts of the `Alt` nodes reached by the single-line walks so far
    alt_infos: HashMap<DocId, AltFlatInfo>,
}

pub(crate) const NEWLINE_DOC: DocId = DocId(0);
pub(crate) const EMPTY_DOC: DocId = DocId(1);
pub(crate) const SPACE_DOC: DocId = DocId(2);
pub(crate) const NEWLINE_OR_SPACE_DOC: DocId = DocId(3);
pub(crate) const EMPTYLINE_DOC: DocId = DocId(4);
pub(crate) const COMMA_DOC: DocId = DocId(5);
pub(crate) const SEMICOLON_DOC: DocId = DocId(6);
pub(crate) const OPEN_BRACE_DOC: DocId = DocId(7);
pub(crate) const CLOSE_BRACE_DOC: DocId = DocId(8);

impl DocArena {
    pub fn new() -> Self {
        let mut arena = DocArena {
            nodes: Vec::new(),
            strings: Vec::new(),
            alt_infos: HashMap::new(),
        };
        let preallocated = [
            arena.make(DocNode::Newline),
            arena.make(DocNode::Empty),
            arena.make_text(" "),
            arena.make(DocNode::Alt(NEWLINE_DOC, SPACE_DOC)),
            arena.make(DocNode::Concat(NEWLINE_DOC, NEWLINE_DOC)),
            arena.make_text(","),
            arena.make_text(";"),
            arena.make_text("{"),
            arena.make_text("}"),
        ];
        assert_eq!(
            preallocated,
            [
                NEWLINE_DOC,
                EMPTY_DOC,
                SPACE_DOC,
                NEWLINE_OR_SPACE_DOC,
                EMPTYLINE_DOC,
                COMMA_DOC,
                SEMICOLON_DOC,
                OPEN_BRACE_DOC,
                CLOSE_BRACE_DOC,
            ]
        );
        arena
    }

    #[inline]
    pub fn make(&mut self, kind: DocNode) -> DocId {
        let id = DocId(self.nodes.len() as u32);
        self.nodes.push(kind);
        id
    }

    /// A `Text` node of `text`
    pub fn make_text(&mut self, text: impl Into<Box<str>>) -> DocId {
        let id = DocStrId(self.strings.len() as u32);
        self.strings.push(text.into());
        self.make(DocNode::Text(id))
    }

    pub fn dump(&self, root: DocId) {
        self.dump_helper(root, 0);
    }

    pub fn get_node(&self, doc: DocId) -> &DocNode {
        &self.nodes[doc.0 as usize]
    }

    pub fn get_text(&self, str_id: DocStrId) -> &str {
        &self.strings[str_id.0 as usize]
    }

    /// Width of the doc on a single line, the narrowest of its alternatives; `None` if it cannot be on
    /// a single line, also because of a text with a line break (a multi-line string literal)
    pub fn flat_width(&mut self, doc: DocId) -> Option<usize> {
        let text_width = |text: &str| {
            (!text.contains('\n')).then(|| unicode_width::UnicodeWidthStr::width(text))
        };
        match self.get_node(doc) {
            DocNode::Token(token) => text_width(token.text()),
            DocNode::Text(t) => text_width(self.get_text(*t)),
            DocNode::Empty | DocNode::AlignMark(_) => Some(0),
            DocNode::Newline | DocNode::Nest(_) | DocNode::Align(_) | DocNode::MultiLine(_) => None,
            &DocNode::Flat(d)
            | &DocNode::KeepTogether(d, _)
            | &DocNode::Preference(d, _)
            | &DocNode::WeakPreference(d, _)
            | &DocNode::ShortLastLine(d)
            | &DocNode::IgnoreOverflow(d) => self.flat_width(d),
            &DocNode::Concat(a, b) => {
                let width1 = self.flat_width(a)?;
                Some(width1 + self.flat_width(b)?)
            }
            DocNode::Alt(..) => self.alt_info(doc).width(),
        }
    }

    /// Whether the doc may have a layout under `Flat`: false only if a line break is forced in all of
    /// its alternatives (unlike `flat_width`, a multi-line token, e.g. a string literal, is allowed, as
    /// the resolver allows it)
    pub fn has_flat_layout(&mut self, doc: DocId) -> bool {
        match self.get_node(doc) {
            DocNode::Newline => false,
            DocNode::Token(_) | DocNode::Text(_) | DocNode::Empty | DocNode::AlignMark(_) => true,
            &DocNode::Nest(d)
            | &DocNode::Align(d)
            | &DocNode::Flat(d)
            | &DocNode::MultiLine(d)
            | &DocNode::KeepTogether(d, _)
            | &DocNode::Preference(d, _)
            | &DocNode::WeakPreference(d, _)
            | &DocNode::ShortLastLine(d)
            | &DocNode::IgnoreOverflow(d) => self.has_flat_layout(d),
            &DocNode::Concat(a, b) => self.has_flat_layout(a) && self.has_flat_layout(b),
            DocNode::Alt(..) => self.alt_info(doc).has_flat_layout(),
        }
    }

    /// The single-line facts of the `Alt` node `doc`, computed on the first call
    fn alt_info(&mut self, doc: DocId) -> AltFlatInfo {
        if let Some(info) = self.alt_infos.get(&doc) {
            return *info;
        }
        let &DocNode::Alt(doc1, doc2) = self.get_node(doc) else {
            panic!("alt_info of a node that is not `Alt`");
        };
        // Width of the narrowest single-line branch
        let width = match (self.flat_width(doc1), self.flat_width(doc2)) {
            (Some(width1), Some(width2)) => Some(width1.min(width2)),
            (width1, width2) => width1.or(width2),
        };
        let has_flat_layout = self.has_flat_layout(doc1) || self.has_flat_layout(doc2);
        let info = AltFlatInfo::new(width, has_flat_layout);
        self.alt_infos.insert(doc, info);
        info
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
                DocNode::Text(text) => {
                    (format!("'{}'", self.get_text(*text)), "box", "lightyellow")
                }
                DocNode::Newline => ("Newline".into(), "rect", "gray"),
                DocNode::Concat(..) => ("Concat".into(), "ellipse", "white"),
                DocNode::Alt(..) => ("Alt".into(), "diamond", "orange"),
                DocNode::Nest(_) => ("Nest".into(), "trapezium", "palegreen"),
                DocNode::Align(_) => ("Align".into(), "parallelogram", "aquamarine"),
                DocNode::Flat(_) => ("Flat".into(), "hexagon", "pink"),
                DocNode::MultiLine(_) => ("MultiLine".into(), "octagon", "plum"),
                DocNode::KeepTogether(_, preference) => (
                    format!("KeepTogether {}", preference.0),
                    "doubleoctagon",
                    "khaki",
                ),
                DocNode::Preference(_, preference) => (
                    format!("Preference {}", preference.0),
                    "tripleoctagon",
                    "khaki",
                ),
                DocNode::WeakPreference(_, preference) => (
                    format!("WeakPreference {}", preference.0),
                    "tripleoctagon",
                    "lightyellow",
                ),
                DocNode::ShortLastLine(_) => ("ShortLastLine".into(), "house", "lightpink"),
                DocNode::IgnoreOverflow(_) => ("IgnoreOverflow".into(), "invhouse", "lightblue"),
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
                DocNode::Alt(d1, d2) => vec![(*d1, ""), (*d2, "")],
                DocNode::Nest(d)
                | DocNode::Align(d)
                | DocNode::Flat(d)
                | DocNode::MultiLine(d)
                | DocNode::KeepTogether(d, _)
                | DocNode::Preference(d, _)
                | DocNode::WeakPreference(d, _)
                | DocNode::ShortLastLine(d)
                | DocNode::IgnoreOverflow(d) => vec![(*d, "")],
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
            DocNode::Preference(d, preference) => {
                println!("Preference[{}](", preference.0);
                self.dump_helper(*d, indent + 1);
                make_ident();
                println!(")");
            }
            DocNode::WeakPreference(d, preference) => {
                println!("WeakPreference[{}](", preference.0);
                self.dump_helper(*d, indent + 1);
                make_ident();
                println!(")");
            }
            DocNode::KeepTogether(d, preference) => {
                println!("KeepTogether[{}](", preference.0);
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
            DocNode::IgnoreOverflow(d) => {
                println!("IgnoreOverflow(");
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
