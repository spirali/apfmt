mod check;
mod comments;
mod config;
mod doc;
mod error;
mod format;
mod formatter;
mod layout;
mod resolver;
mod rewrite;
mod utils;

use crate::check::check_result;
pub use crate::config::Config;
use crate::format::format_source_file;
use crate::formatter::{Formatter, Unsupported};
use crate::rewrite::rewrite_source;
use crate::utils::line_column;
pub use error::{Error, Result, UnsupportedSyntax};
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

fn parse_source_code(config: &Config, code: &str) -> Result<SourceFile> {
    let parse = SourceFile::parse(code, config.edition);
    let errors = parse.errors();
    if !errors.is_empty() {
        return Err(Error::SyntaxErrors(errors));
    }
    Ok(parse.tree())
}

/// The parsed source and the tree to format: the source after the rewrites (`rewrite/`, e.g. the imports
/// sorted), parsed again if they changed it
fn prepare_source_code(config: &Config, code: &str) -> Result<(SourceFile, SourceFile)> {
    let file = parse_source_code(config, code)?;
    let prepared = match rewrite_source(&file, code) {
        Some(rewritten) => parse_source_code(config, &rewritten).map_err(|error| {
            Error::GenericError(format!("Rewriting the source broke the code: {error:?}"))
        })?,
        None => file.clone(),
    };
    if config.dump_parse_tree {
        print_tree(&prepared);
    }
    Ok((file, prepared))
}

/// The doc graph built for the source code in the Graphviz DOT format, instead of the formatted code
pub fn doc_graph_to_dot(config: &Config, code: &str) -> Result<String> {
    let (_, file) = prepare_source_code(config, code)?;
    let fmt = Formatter::new(config);
    let frag = format_source_file(&fmt, &file);
    Ok(fmt.doc_graph_to_dot(&frag))
}

/// The formatted code
pub struct Formatted {
    pub code: String,
    /// The source text copied unchanged into `code` because afmt cannot format it yet (only with
    /// `Config::allow_unsupported`, which makes it an error otherwise)
    pub unsupported: Vec<UnsupportedSyntax>,
}

/// Where the text copied by `Formatter::unsupported` is in `source`, in the source order; a node in
/// more layouts is listed once
fn locate_unsupported(
    source: &SourceFile,
    mut unsupported: Vec<Unsupported>,
) -> Vec<UnsupportedSyntax> {
    if unsupported.is_empty() {
        return Vec::new();
    }
    let source = source.syntax().to_string();
    unsupported.sort_by_key(|u| (u.range.start(), u.range.end()));
    unsupported.dedup_by_key(|u| u.range);
    unsupported
        .into_iter()
        .map(|u| {
            let (line, column) = line_column(&source, u.range.start());
            UnsupportedSyntax {
                line,
                column,
                what: u.what,
                snippet: source[u.range]
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .to_string(),
            }
        })
        .collect()
}

pub fn format_source_code(config: &Config, code: &str) -> Result<Formatted> {
    let (file, prepared) = prepare_source_code(config, code)?;

    let fmt = Formatter::new(config);
    let frag = format_source_file(&fmt, &prepared);
    let unsupported = locate_unsupported(&prepared, fmt.take_unsupported());
    if !unsupported.is_empty() && !config.allow_unsupported {
        return Err(Error::Unsupported(unsupported));
    }

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

    Ok(Formatted {
        code: out,
        unsupported,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn very_long_line_does_not_overflow_badness() {
        let literal = format!("\"{}\"", "x".repeat(70_000));
        let code = format!("fn f() {{ g({literal}, 1); }}");
        let out = format_source_code(&Config::default(), &code).unwrap().code;
        assert_eq!(
            out,
            format!("fn f() {{\n    g(\n        {literal},\n        1,\n    );\n}}")
        );
    }

    const UNSUPPORTED: &str = "fn f() {\n    let   v: m!() = 1;\n    try { 1 }\n}";

    #[test]
    fn unsupported_is_an_error_by_default() {
        let Err(Error::Unsupported(unsupported)) =
            format_source_code(&Config::default(), UNSUPPORTED)
        else {
            panic!("not reported");
        };
        let found = unsupported
            .iter()
            .map(|u| (u.line, u.column, u.what.as_str(), u.snippet.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(
            found,
            [
                (2, 14, "MACRO_TYPE", "m!()"),
                (3, 5, "a `try` block", "try { 1 }"),
            ]
        );
    }

    #[test]
    fn unsupported_is_copied_when_allowed() {
        let config = Config {
            allow_unsupported: true,
            check_result: true,
            ..Config::default()
        };
        let formatted = format_source_code(&config, UNSUPPORTED).unwrap();
        assert_eq!(
            formatted.code,
            "fn f() {\n    let v: m!() = 1;\n    try { 1 }\n}"
        );
        assert_eq!(formatted.unsupported.len(), 2);
    }

    #[test]
    fn trailing_comment_align_threshold() {
        let code = "struct S {\n    a: u8, // a\n    bb: u8, // b\n    cccccccccc: String, // c\n}";
        let all = "struct S {\n    a: u8,              // a\n    bb: u8,             // b\n    cccccccccc: String, // c\n}";
        let without_widest =
            "struct S {\n    a: u8,  // a\n    bb: u8, // b\n    cccccccccc: String, // c\n}";
        assert_eq!(
            format_source_code(&Config::default(), code).unwrap().code,
            all
        );
        for (threshold, expected) in [(12, all), (11, without_widest), (0, code)] {
            let config = Config {
                trailing_comment_align_threshold: threshold,
                ..Config::default()
            };
            assert_eq!(format_source_code(&config, code).unwrap().code, expected);
        }
    }

    #[test]
    fn normalize_comments() {
        let code = "fn/*a*/f() { g(/**/ 1 /*  b  */, /** c*/ 2); }";
        let config = Config::default();
        assert_eq!(
            format_source_code(&config, code).unwrap().code,
            "fn /* a */ f() {\n    g(/**/ 1 /* b */, /** c*/ 2);\n}"
        );
        let config = Config {
            normalize_comments: false,
            ..Config::default()
        };
        assert_eq!(
            format_source_code(&config, code).unwrap().code,
            "fn /*a*/ f() {\n    g(/**/ 1 /*  b  */, /** c*/ 2);\n}"
        );
    }
}
