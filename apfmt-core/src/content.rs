use crate::doc::{AlignGroup, DocId};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct ContentId(u32);

#[derive(Clone, Copy)]
pub enum Content {
    Text(DocId),
    Newline,
    Concat(ContentId, ContentId),
    Indent(u32, ContentId),
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
