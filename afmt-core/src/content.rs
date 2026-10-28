use crate::doc::{AlignGroup, DocId};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct ContentId(u32);

/// The number of block indentation levels (`DocNode::Nest`) in an indentation
pub type Levels = u16;

/// The indentation of a line: `cols` columns, the first `levels × tab_spaces` of them block
/// indentation (`DocNode::Nest`), the rest alignment (`DocNode::Align`); with `hard_tabs`, the block
/// indentation is written as tabs and the alignment as spaces. `levels` is counted only with
/// `hard_tabs` (0 otherwise).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Indentation {
    pub cols: u32,
    pub levels: Levels,
}

#[derive(Clone, Copy)]
pub enum Content {
    Text(DocId),
    Newline,
    Concat(ContentId, ContentId),
    /// The fields of an `Indentation`, flat so that the tag of `Content` fits in the padding
    Indent(u32, Levels, ContentId),
    Mark(AlignGroup),
}

pub(crate) struct ContentArena {
    objects: Vec<Content>,
}

pub(crate) const CONTENT_NEWLINE: ContentId = ContentId(0);

impl ContentArena {
    pub fn new() -> Self {
        let mut objects = Vec::with_capacity(1024);
        objects.push(Content::Newline);
        ContentArena { objects }
    }

    #[inline]
    pub fn make(&mut self, content: Content) -> ContentId {
        match content {
            Content::Newline => CONTENT_NEWLINE,
            _ => {
                let len = self.objects.len();
                self.objects.push(content);
                ContentId(len as u32)
            }
        }
    }

    pub fn get(&self, id: ContentId) -> Content {
        self.objects[id.0 as usize]
    }
}
