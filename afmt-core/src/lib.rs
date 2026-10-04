mod comments;
mod config;
mod doc;
mod error;
mod format;
mod formatter;
mod list;
mod measure;
mod resolver;
mod utils;

pub use crate::config::Config;
use crate::format::format_source_file;
use crate::formatter::Formatter;
use ra_ap_syntax::{AstNode, SyntaxError, WalkEvent};
use ra_ap_syntax::{Edition, NodeOrToken, SourceFile};

fn print_tree(source_file: &SourceFile) {
    let mut depth = 0;
    for walk in source_file.syntax().preorder_with_tokens() {
        match walk {
            WalkEvent::Enter(node) => {
                for _ in 0..depth {
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
                NodeOrToken::Node(_) => {
                    depth -= 1;
                    for _ in 0..depth {
                        print!("    ");
                    }
                    println!("}}");
                }
                NodeOrToken::Token(_) => {}
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

    let mut fmt = Formatter::new(config);
    let frag = format_source_file(&mut fmt, file);
    let out = fmt.into_resolver(frag).render(code.len() * 2);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_comments() {
        let code = "fn/*a*/f() { g(/**/ 1 /*  b  */, /** c*/ 2); }";
        let config = Config::default();
        assert_eq!(
            format_source_code(&config, code).unwrap(),
            "fn /* a */ f() {\n    g(/**/ 1, /* b */ /** c*/ 2);\n}"
        );
        let config = Config {
            normalize_comments: false,
            ..Config::default()
        };
        assert_eq!(
            format_source_code(&config, code).unwrap(),
            "fn /*a*/ f() {\n    g(/**/ 1, /*  b  */ /** c*/ 2);\n}"
        );
    }
}
