#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Measure {
    height: u32,
    last_width: u32,
    max_width: u32,
}

impl Measure {
    pub fn last_width(&self) -> u32 {
        self.last_width
    }

    pub fn expand_last(&mut self, size: u32) {
        self.last_width += size;
        self.max_width = self.max_width.max(self.last_width);
    }

    pub fn text(s: &str) -> Self {
        Measure {
            height: 0,
            last_width: s.len() as u32,
            max_width: s.len() as u32,
        }
    }

    pub fn new_line(&mut self) {
        self.height += 1;
        self.last_width = 0;
    }

    pub fn concat(&self, other: &Measure) -> Measure {
        Measure {
            height: self.height + other.height,
            last_width: self.last_width + other.last_width,
            max_width: self.max_width.max(self.last_width + other.max_width),
        }
    }

    pub fn is_valid(&self, page_width: u32) -> bool {
        self.max_width <= page_width
    }

    pub fn dominates(&self, other: &Measure) -> bool {
        self.height <= other.height
            && self.max_width <= other.max_width
            && self.last_width <= other.last_width
    }
}
