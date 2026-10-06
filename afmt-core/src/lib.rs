mod check;
mod comments;
mod config;
mod doc;
mod error;
mod format;
mod formatter;
mod layout;
mod resolver;
mod utils;

use crate::check::check_result;
pub use crate::config::Config;
use crate::format::format_source_file;
use crate::formatter::Formatter;
pub use error::{Error, Result};
use ra_ap_syntax::{AstNode, WalkEvent};
use ra_ap_syntax::{NodeOrToken, SourceFile};

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

pub fn format_source_code(config: &Config, code: &str) -> Result<String> {
    let parse = SourceFile::parse(code, config.edition);
    let file: SourceFile = parse.tree();

    let errors = parse.errors();
    if !errors.is_empty() {
        return Err(Error::SyntaxErrors(parse.errors()));
    }

    if config.dump_parse_tree {
        print_tree(&file);
    }

    let fmt = Formatter::new(config);
    let frag = format_source_file(&fmt, &file);

    if config.dump_doc_tree {
        fmt.dump(&frag);
    }

    let out = fmt.into_resolver(frag).render(code.len() * 2);

    if config.check_result {
        let out_parse = SourceFile::parse(&out, config.edition);
        let errors = out_parse.errors();
        if !errors.is_empty() {
            return Err(Error::GenericError(format!(
                "Could not reparse the result: {:?}",
                errors
            )));
        }
        if let Err(error) = check_result(&file, &out_parse.tree(), code, &out) {
            return Err(Error::GenericError(error));
        }
    }

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
            "fn /* a */ f() {\n    g(/**/ 1 /* b */, /** c*/ 2);\n}"
        );
        let config = Config {
            normalize_comments: false,
            ..Config::default()
        };
        assert_eq!(
            format_source_code(&config, code).unwrap(),
            "fn /*a*/ f() {\n    g(/**/ 1 /*  b  */, /** c*/ 2);\n}"
        );
    }
}
