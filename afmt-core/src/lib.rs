mod builder;
mod config;
mod doc;
mod error;
mod formatter;
mod measure;
mod resolver;

use crate::builder::OutputBuilder;
pub use crate::config::Config;
use crate::formatter::{Formatter, format_source_file};
use ra_ap_syntax::{AstNode, SyntaxError, WalkEvent};
use ra_ap_syntax::{Edition, NodeOrToken, SourceFile, SyntaxElement, SyntaxKind};
use std::path::Path;

fn print_tree(source_file: &SourceFile) {
    let mut depth = 0;
    for walk in source_file.syntax().preorder_with_tokens() {
        match walk {
            WalkEvent::Enter(node) => {
                for i in 0..depth {
                    print!("    ");
                }
                match node {
                    NodeOrToken::Node(node) => {
                        depth += 1;
                        println!("{:?} {}", node, node.index());
                    }
                    NodeOrToken::Token(token) => {
                        println!(
                            ">> {:?} text={:?} index={}",
                            token.kind(),
                            token.text(),
                            token.index(),
                        );
                    }
                }
            }
            WalkEvent::Leave(node) => match node {
                NodeOrToken::Node(node) => {
                    depth -= 1;
                    for i in 0..depth {
                        print!("    ");
                    }
                    println!("}}");
                }
                NodeOrToken::Token(token) => {}
            },
        }
    }
}

pub fn format_source_code(config: &Config, code: &str) -> Result<String, Vec<SyntaxError>> {
    let parse = SourceFile::parse(&code, Edition::CURRENT);
    let file: SourceFile = parse.tree();

    let errors = parse.errors();
    if !errors.is_empty() {
        return Err(parse.errors());
    }

    if config.dump_tree {
        print_tree(&file);
    }

    //println!("---");

    //let output = OutputBuilder::new(text.len() * 2);
    //let input = InputStream::new(file);

    let mut fmt = Formatter::new(config);
    let doc = format_source_file(&mut fmt, file);
    //fmt.dump(doc);
    //dbg!(&doc);
    //fmt.dump(doc);
    let out = fmt.into_resolver(doc).render(code.len() * 2);
    //println!("{}", out);
    Ok(out)
    /*for event in file.syntax().preorder_with_tokens() {
            match event {
                WalkEvent::Enter(node_or_tok) => {
                    match node_or_tok {
                        SyntaxElement::Token(tok) => {
                            match tok.kind() {
                                SyntaxKind::WHITESPACE | SyntaxKind::NEWLINE => {
                                    //output.put_space();
                                }
                                SyntaxKind::SEMICOLON | SyntaxKind::L_CURLY | SyntaxKind::R_CURLY => {
                                    output.put_str(tok.text());
                                    output.put_char('\n');
                                }

                                SyntaxKind::IDENT => {
                                    output.put_str(tok.text());
                                }

                                _ => {
                                    output.put_str(tok.text());
                                    output.put_space();
                                }
                            }
                            println!("TOK {:indent$} {:?} {:?}", "", tok.kind(), tok.text(), indent = 0);
                        }
                        SyntaxElement::Node(node) => {
                            println!("NODE {:indent$}{:?}", "", node.kind(), indent = 0);
                        }
                    }
                }
                WalkEvent::Leave(node_or_tok) => {
                    //println!("LEAVE {:indent$}{:?}", "", node_or_tok.kind(), indent = 0);
                }
            }
        }
    */
}
