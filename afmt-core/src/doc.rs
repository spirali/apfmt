use rustc_hash::{FxHashMap, FxHashSet};
use std::fmt::Write;

use crate::utils::text_width;
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

/// The resolver ranks layouts by the overflow, then by the lines times `LINE_COST` plus their
/// `Cost`s
pub(crate) const LINE_COST: i32 = 10;

/// More than any difference in lines between the layouts of real code
pub(crate) const SPLIT_COST: i32 = 1000 * LINE_COST;

/// What a layout costs on top of its lines, in the units of `LINE_COST`: mostly whole splits
#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub(crate) struct Cost(i32);

impl Cost {
    pub const SPLIT: Cost = Cost::splits(1);

    #[inline]
    pub const fn new(value: i32) -> Self {
        Cost(value)
    }

    #[inline]
    pub const fn splits(count: i32) -> Self {
        Cost(count * SPLIT_COST)
    }
}

impl From<Cost> for i64 {
    #[inline]
    fn from(cost: Cost) -> i64 {
        i64::from(cost.0)
    }
}

/// Preference among layouts with the same overflow and cost: positive is preferred, negative
/// dispreferred; never mixed with a `Cost`
#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub(crate) struct WeakPreference(i32);

impl WeakPreference {
    pub const NEUTRAL: WeakPreference = WeakPreference(0);
    pub const PREFERRED: WeakPreference = WeakPreference(1);

    pub fn preferred_by(n: u32) -> Self {
        WeakPreference(i32::try_from(n).unwrap_or(i32::MAX))
    }

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
    Fallback(DocId, DocId),
    Nest(DocId),
    Align(DocId),
    Flat(DocId),
    MultiLine(DocId),
    /// Layouts spanning more lines get the cost (e.g. a split)
    KeepTogether(DocId, Cost),
    /// Every layout gets the cost, compared right after the overflow, e.g. a split for a layout the
    /// Style Guide accepts only as a fallback
    Cost(DocId, Cost),
    /// Every layout gets the preference, compared only after the cost: decides between otherwise
    /// equal layouts
    WeakPreference(DocId, WeakPreference),
    /// Only layouts whose last line ends at most at the indentation of a nested line, e.g. `x.baz?`
    ShortLastLine(DocId),
    /// Only layouts whose last line ends at least this many columns before the page width: room for
    /// text that always follows, e.g. ` {` after a match arm pattern
    RoomAfter(DocId, u32),
    /// Only layouts whose first line spans at most this many columns from the start, e.g. a width
    /// heuristic (`fn_call_width`) under `Flat`
    FirstLineWithin(DocId, u32),
    /// Overflow beyond the page width does not count, e.g. a trailing comment, so it does not
    /// change the layout of the code in front of it
    IgnoreOverflow(DocId),
    AlignMark(AlignGroup),
}

/// What the walks of `DocArena` know about an `Alt` node, computed once: a shared doc is then
/// walked once, not once per path to it (exponential in the nesting)
#[derive(Clone, Copy, Debug)]
pub(crate) struct AltInfo {
    flags: AltFlags,
    /// If `AltFlags::HAS_SINGLE_LINE`; plain `u32`s keep `AltInfo` at 16 bytes
    min_flat_width: u32,
    /// If `AltFlags::HAS_NATURAL_FLAT` / `AltFlags::HAS_NATURAL_FIRST_LINE`
    natural_flat: u32,
    natural_first_line: u32,
}

impl AltInfo {
    fn min_flat_width(self) -> Option<u32> {
        (self.flags.contains(AltFlags::HAS_SINGLE_LINE)).then_some(self.min_flat_width)
    }

    fn natural_widths(self) -> NaturalWidths {
        NaturalWidths {
            flat: (self.flags.contains(AltFlags::HAS_NATURAL_FLAT)).then_some(self.natural_flat),
            first_line: (self.flags.contains(AltFlags::HAS_NATURAL_FIRST_LINE))
                .then_some(self.natural_first_line),
        }
    }
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug)]
    struct AltFlags: u8 {
        const HAS_FLAT_LAYOUT = 1;
        const HAS_MULTI_LINE_TEXT = 1 << 1;
        const HAS_LINE_BREAK = 1 << 2;
        const HAS_SINGLE_LINE = 1 << 3;
        const HAS_NATURAL_FLAT = 1 << 4;
        const HAS_NATURAL_FIRST_LINE = 1 << 5;
    }
}

pub(crate) struct DocArena {
    nodes: Vec<DocNode>,
    strings: Vec<Box<str>>,
    alt_infos: FxHashMap<DocId, AltInfo>,
    has_align_marks: bool,
}

/// The narrowest lines of a doc in its layouts that take no dispreferred choice on them
#[derive(Clone, Copy, Debug)]
struct NaturalWidths {
    flat: Option<u32>,
    /// The narrowest first line of a layout with a line break, up to it
    first_line: Option<u32>,
}

impl NaturalWidths {
    const NONE: NaturalWidths = NaturalWidths {
        flat: None,
        first_line: None,
    };

    fn min(self, other: NaturalWidths) -> NaturalWidths {
        let min = |a: Option<u32>, b: Option<u32>| match (a, b) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        NaturalWidths {
            flat: min(self.flat, other.flat),
            first_line: min(self.first_line, other.first_line),
        }
    }
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
/// Kept by `DocArena::clear`
const PREALLOCATED_NODES: usize = 9;
const PREALLOCATED_STRINGS: usize = 5;

impl DocArena {
    pub fn new() -> Self {
        let mut arena = DocArena {
            nodes: Vec::new(),
            strings: Vec::new(),
            alt_infos: FxHashMap::default(),
            has_align_marks: false,
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
        assert_eq!(
            (arena.nodes.len(), arena.strings.len()),
            (PREALLOCATED_NODES, PREALLOCATED_STRINGS)
        );
        arena
    }

    /// Removes every node but the preallocated ones; the `DocId`s made before are invalid after it
    pub fn clear(&mut self) {
        self.nodes.truncate(PREALLOCATED_NODES);
        self.strings.truncate(PREALLOCATED_STRINGS);
        self.alt_infos.clear();
        self.has_align_marks = false;
    }

    pub fn has_align_marks(&self) -> bool {
        self.has_align_marks
    }

    #[inline]
    pub fn make(&mut self, kind: DocNode) -> DocId {
        let id = DocId(self.nodes.len() as u32);
        self.has_align_marks |= matches!(kind, DocNode::AlignMark(_));
        self.nodes.push(kind);
        id
    }

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

    /// The narrowest single line without any line break (also not in a token); `None` if there is
    /// none. Exact (except `ShortLastLine` and `RoomAfter`), not the width of the layout placed.
    pub fn min_flat_width(&mut self, doc: DocId) -> Option<u32> {
        let text_width = |text: &str| (!text.contains('\n')).then(|| text_width(text));
        match self.get_node(doc) {
            DocNode::Token(token) => text_width(token.text()),
            DocNode::Text(t) => text_width(self.get_text(*t)),
            DocNode::Empty | DocNode::AlignMark(_) => Some(0),
            DocNode::Newline | DocNode::MultiLine(_) => None,
            &DocNode::Flat(d)
            | &DocNode::Nest(d)
            | &DocNode::Align(d)
            | &DocNode::KeepTogether(d, _)
            | &DocNode::Cost(d, _)
            | &DocNode::WeakPreference(d, _)
            | &DocNode::ShortLastLine(d)
            | &DocNode::RoomAfter(d, _)
            | &DocNode::IgnoreOverflow(d) => self.min_flat_width(d),
            &DocNode::FirstLineWithin(d, width) => self.min_flat_width(d).filter(|&w| w <= width),
            &DocNode::Concat(a, b) => {
                let width1 = self.min_flat_width(a)?;
                Some(width1.saturating_add(self.min_flat_width(b)?))
            }
            DocNode::Alt(..) | DocNode::Fallback(..) => self.alt_info(doc).min_flat_width(),
        }
    }

    /// False only if a line break is forced in all alternatives (a multi-line token is allowed), or
    /// a `FirstLineWithin` cannot fit
    pub fn has_flat_layout(&mut self, doc: DocId) -> bool {
        match self.get_node(doc) {
            DocNode::Newline => false,
            DocNode::Token(_) | DocNode::Text(_) | DocNode::Empty | DocNode::AlignMark(_) => true,
            &DocNode::Nest(d)
            | &DocNode::Align(d)
            | &DocNode::Flat(d)
            | &DocNode::MultiLine(d)
            | &DocNode::KeepTogether(d, _)
            | &DocNode::Cost(d, _)
            | &DocNode::WeakPreference(d, _)
            | &DocNode::ShortLastLine(d)
            | &DocNode::RoomAfter(d, _)
            | &DocNode::IgnoreOverflow(d) => self.has_flat_layout(d),
            &DocNode::FirstLineWithin(d, width) => {
                self.min_flat_width(d).is_some_and(|w| w <= width)
            }
            &DocNode::Concat(a, b) => self.has_flat_layout(a) && self.has_flat_layout(b),
            DocNode::Alt(..) | DocNode::Fallback(..) => {
                self.alt_flag(doc, AltFlags::HAS_FLAT_LAYOUT)
            }
        }
    }

    /// A multi-line string literal, or source copied unchanged
    pub fn has_multi_line_text(&mut self, doc: DocId) -> bool {
        match self.get_node(doc) {
            DocNode::Token(token) => token.text().contains('\n'),
            DocNode::Text(t) => self.get_text(*t).contains('\n'),
            DocNode::Empty | DocNode::AlignMark(_) | DocNode::Newline => false,
            &DocNode::Nest(d)
            | &DocNode::Align(d)
            | &DocNode::Flat(d)
            | &DocNode::MultiLine(d)
            | &DocNode::KeepTogether(d, _)
            | &DocNode::Cost(d, _)
            | &DocNode::WeakPreference(d, _)
            | &DocNode::ShortLastLine(d)
            | &DocNode::RoomAfter(d, _)
            | &DocNode::IgnoreOverflow(d)
            | &DocNode::FirstLineWithin(d, _) => self.has_multi_line_text(d),
            &DocNode::Concat(a, b) => self.has_multi_line_text(a) || self.has_multi_line_text(b),
            DocNode::Alt(..) | DocNode::Fallback(..) => {
                self.alt_flag(doc, AltFlags::HAS_MULTI_LINE_TEXT)
            }
        }
    }

    /// Outside `Flat`
    pub fn has_line_break(&mut self, doc: DocId) -> bool {
        match self.get_node(doc) {
            DocNode::Newline => true,
            DocNode::Token(_)
            | DocNode::Text(_)
            | DocNode::Empty
            | DocNode::AlignMark(_)
            | DocNode::Flat(_) => false,
            &DocNode::Nest(d)
            | &DocNode::Align(d)
            | &DocNode::MultiLine(d)
            | &DocNode::KeepTogether(d, _)
            | &DocNode::Cost(d, _)
            | &DocNode::WeakPreference(d, _)
            | &DocNode::ShortLastLine(d)
            | &DocNode::RoomAfter(d, _)
            | &DocNode::IgnoreOverflow(d)
            | &DocNode::FirstLineWithin(d, _) => self.has_line_break(d),
            &DocNode::Concat(a, b) => self.has_line_break(a) || self.has_line_break(b),
            DocNode::Alt(..) | DocNode::Fallback(..) => {
                self.alt_flag(doc, AltFlags::HAS_LINE_BREAK)
            }
        }
    }

    /// The narrowest first line of a broken layout that takes no dispreferred choice up to it: how
    /// wide it is when nothing is squeezed into a narrower one at a cost (`match x` / `{`)
    pub fn natural_first_line_width(&mut self, doc: DocId) -> Option<u32> {
        self.natural_widths(doc).first_line
    }

    fn natural_widths(&mut self, doc: DocId) -> NaturalWidths {
        let text = |text: &str| match text.split_once('\n') {
            None => NaturalWidths {
                flat: Some(text_width(text)),
                first_line: None,
            },
            Some((first, _)) => NaturalWidths {
                flat: None,
                first_line: Some(text_width(first)),
            },
        };
        match self.get_node(doc) {
            DocNode::Token(token) => text(token.text()),
            DocNode::Text(t) => text(self.get_text(*t)),
            DocNode::Empty | DocNode::AlignMark(_) => NaturalWidths {
                flat: Some(0),
                first_line: None,
            },
            DocNode::Newline => NaturalWidths {
                flat: None,
                first_line: Some(0),
            },
            &DocNode::Nest(d)
            | &DocNode::Align(d)
            | &DocNode::WeakPreference(d, _)
            | &DocNode::ShortLastLine(d)
            | &DocNode::RoomAfter(d, _)
            | &DocNode::IgnoreOverflow(d) => self.natural_widths(d),
            &DocNode::Cost(d, cost) => {
                if cost.0 > 0 {
                    NaturalWidths::NONE
                } else {
                    self.natural_widths(d)
                }
            }
            &DocNode::KeepTogether(d, cost) => {
                let widths = self.natural_widths(d);
                NaturalWidths {
                    first_line: widths.first_line.filter(|_| cost.0 <= 0),
                    ..widths
                }
            }
            &DocNode::Flat(d) => NaturalWidths {
                first_line: None,
                ..self.natural_widths(d)
            },
            &DocNode::MultiLine(d) => NaturalWidths {
                flat: None,
                ..self.natural_widths(d)
            },
            &DocNode::FirstLineWithin(d, width) => {
                let widths = self.natural_widths(d);
                NaturalWidths {
                    flat: widths.flat.filter(|&w| w <= width),
                    first_line: widths.first_line.filter(|&w| w <= width),
                }
            }
            &DocNode::Concat(a, b) => {
                let first = self.natural_widths(a);
                let Some(flat) = first.flat else {
                    return NaturalWidths {
                        flat: None,
                        first_line: first.first_line,
                    };
                };
                let second = self.natural_widths(b);
                let after = |w: Option<u32>| w.map(|w| flat.saturating_add(w));
                NaturalWidths {
                    flat: after(second.flat),
                    first_line: first.first_line,
                }
                .min(NaturalWidths {
                    flat: None,
                    first_line: after(second.first_line),
                })
            }
            DocNode::Alt(..) | DocNode::Fallback(..) => self.alt_info(doc).natural_widths(),
        }
    }

    fn alt_flag(&mut self, doc: DocId, flag: AltFlags) -> bool {
        self.alt_info(doc).flags.contains(flag)
    }

    /// Computed on the first call
    fn alt_info(&mut self, doc: DocId) -> AltInfo {
        if let Some(info) = self.alt_infos.get(&doc) {
            return *info;
        }
        let (&DocNode::Alt(doc1, doc2) | &DocNode::Fallback(doc1, doc2)) = self.get_node(doc)
        else {
            panic!("alt_info of a node that is not `Alt` or `Fallback`");
        };
        let mut flags = AltFlags::empty();
        flags.set(
            AltFlags::HAS_FLAT_LAYOUT,
            self.has_flat_layout(doc1) || self.has_flat_layout(doc2),
        );
        flags.set(
            AltFlags::HAS_MULTI_LINE_TEXT,
            self.has_multi_line_text(doc1) || self.has_multi_line_text(doc2),
        );
        flags.set(
            AltFlags::HAS_LINE_BREAK,
            self.has_line_break(doc1) || self.has_line_break(doc2),
        );
        let min_flat_width = match (self.min_flat_width(doc1), self.min_flat_width(doc2)) {
            (Some(width1), Some(width2)) => Some(width1.min(width2)),
            (width1, width2) => width1.or(width2),
        };
        flags.set(AltFlags::HAS_SINGLE_LINE, min_flat_width.is_some());
        let natural = self.natural_widths(doc1).min(self.natural_widths(doc2));
        flags.set(AltFlags::HAS_NATURAL_FLAT, natural.flat.is_some());
        flags.set(
            AltFlags::HAS_NATURAL_FIRST_LINE,
            natural.first_line.is_some(),
        );
        let info = AltInfo {
            flags,
            min_flat_width: min_flat_width.unwrap_or(0),
            natural_flat: natural.flat.unwrap_or(0),
            natural_first_line: natural.first_line.unwrap_or(0),
        };
        self.alt_infos.insert(doc, info);
        info
    }

    /// Shared nodes are written once
    pub fn to_dot(&self, root: DocId) -> String {
        let mut out = String::from(
            "digraph doc {\n    ordering=out;\n    node [style=filled, fontname=\"monospace\"];\n",
        );
        let mut visited = FxHashSet::default();
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
                DocNode::Fallback(..) => ("Fallback".into(), "diamond", "gold"),
                DocNode::Nest(_) => ("Nest".into(), "trapezium", "palegreen"),
                DocNode::Align(_) => ("Align".into(), "parallelogram", "aquamarine"),
                DocNode::Flat(_) => ("Flat".into(), "hexagon", "pink"),
                DocNode::MultiLine(_) => ("MultiLine".into(), "octagon", "plum"),
                DocNode::KeepTogether(_, cost) => {
                    (format!("KeepTogether {}", cost.0), "doubleoctagon", "khaki")
                }
                DocNode::Cost(_, cost) => (format!("Cost {}", cost.0), "tripleoctagon", "khaki"),
                DocNode::WeakPreference(_, preference) => (
                    format!("WeakPreference {}", preference.0),
                    "tripleoctagon",
                    "lightyellow",
                ),
                DocNode::ShortLastLine(_) => ("ShortLastLine".into(), "house", "lightpink"),
                DocNode::RoomAfter(_, room) => (format!("RoomAfter {room}"), "house", "lightpink"),
                DocNode::FirstLineWithin(_, width) => {
                    (format!("FirstLineWithin {width}"), "invhouse", "lightpink")
                }
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
                DocNode::Fallback(d1, d2) => vec![(*d1, "1"), (*d2, "2")],
                DocNode::Nest(d)
                | DocNode::Align(d)
                | DocNode::Flat(d)
                | DocNode::MultiLine(d)
                | DocNode::KeepTogether(d, _)
                | DocNode::Cost(d, _)
                | DocNode::WeakPreference(d, _)
                | DocNode::ShortLastLine(d)
                | DocNode::RoomAfter(d, _)
                | DocNode::FirstLineWithin(d, _)
                | DocNode::IgnoreOverflow(d) => vec![(*d, "")],
            };
            for (child, edge_label) in &children {
                write!(out, "    n{} -> n{}", doc.0, child.0).unwrap();
                if !edge_label.is_empty() {
                    write!(out, " [label=\"{}\"]", edge_label).unwrap();
                }
                out.push_str(";\n");
            }
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
            DocNode::Fallback(d1, d2) => {
                println!("Fallback(");
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
            DocNode::Cost(d, cost) => {
                println!("Cost[{}](", cost.0);
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
            DocNode::RoomAfter(d, room) => {
                println!("RoomAfter[{room}](");
                self.dump_helper(*d, indent + 1);
                make_ident();
                println!(")");
            }
            DocNode::FirstLineWithin(d, width) => {
                println!("FirstLineWithin[{width}](");
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
