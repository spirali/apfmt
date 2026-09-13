use crate::config::Config;
use crate::doc::{Doc, DocId};
use std::path::PathBuf;
use std::rc::Rc;

pub(crate) struct OutputBuilder {
    str: String,
    indent: usize,
    tab_size: usize,
    doc: Option<Doc>,
}

impl OutputBuilder {
    pub fn new() -> Self {
        OutputBuilder {
            str: String::new(),
            indent: 0,
            tab_size: 4,
            doc: None,
        }
    }

    pub fn push_indent(&mut self) {
        self.indent += 1;
        self.new_line();
    }

    pub fn pop_indent(&mut self) {
        assert!(self.indent > 0);
        self.indent -= 1;
        self.new_line();
    }

    pub fn new_line(&mut self) {
        self.str.push('\n');
        for _ in 0..self.indent * self.tab_size {
            self.str.push(' ');
        }
    }

    pub fn add_char(&mut self, c: char) {
        self.str.push(c);
    }

    pub fn add_text(&mut self, text: &str) {
        /*if !self.ends_with_whitespace {
            self.str.push(' ');
        }*/
        self.str.push_str(text);
        //self.ends_with_whitespace = false;
    }

    pub fn finish(self) -> String {
        self.str
    }
}
/*
pub(crate) struct DocBuilder {
    doc: DocId,
}

impl DocBuilder {
    pub fn new(doc: DocId) -> Self {
        DocBuilder {
            doc,
        }
    }

    pub fn concat(&mut self, doc: &DocId) {
    }

    pub fn finish(self) -> Doc {
    }

    pub fn add_text(&mut self, text: &str) {
    }

    pub fn add_char(&mut self, c: char) {
        if self.fresh_line {
            self.fresh_line = false;
            self.doc.add_ident(self.ident);
        }
        self.doc.add_char(self.cfg.max_width, c);
    }

    pub fn push_indent(&mut self) {
        self.ident += self.cfg.indent_width;
    }

    pub fn pop_indent(&mut self) {
        self.ident -= self.cfg.indent_width;
    }

    pub fn new_line(&mut self) {
        self.doc.new_line();
        self.fresh_line = true;
    }
}
*/
