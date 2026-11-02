mod check;
mod comments;
mod config;
mod content;
mod doc;
mod error;
mod format;
mod formatter;
mod layout;
mod modules;
mod resolver;
mod rewrite;
mod syntax_utils;
#[cfg(test)]
mod test_utils;
mod utils;

use crate::check::check_result;
pub use crate::config::{Config, NewlineStyle};
use crate::format::format_source_file;
use crate::formatter::{Builder, Formatter, Unsupported};
use crate::rewrite::rewrite_source;
use crate::syntax_utils::LineColumns;
pub use config::cargo::{CARGO_MANIFEST_NAME, find_cargo_manifest};
pub use error::{Error, Result, UnsupportedSyntax};
pub use ra_ap_syntax::Edition;
use ra_ap_syntax::{AstNode, WalkEvent};
use ra_ap_syntax::{NodeOrToken, SourceFile};
use std::borrow::Cow;

pub use crate::config::source::{ConfigOptions, config_for_source};
pub use crate::modules::{ModuleFiles, module_files};
pub use crate::utils::canonical_path;

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

/// The parsed source, the tree to format and its text: the source after the rewrites, parsed again
/// if changed
fn prepare_source_code<'a>(
    config: &Config,
    code: &'a str,
) -> Result<(SourceFile, SourceFile, Cow<'a, str>)> {
    let file = parse_source_code(config, code)?;
    let (prepared, text) = match rewrite_source(&file, code) {
        Some(rewritten) => {
            let prepared = parse_source_code(config, &rewritten).map_err(|error| {
                Error::GenericError(format!("Rewriting the source broke the code: {error:?}"))
            })?;
            (prepared, Cow::Owned(rewritten))
        }
        None => (file.clone(), Cow::Borrowed(code)),
    };
    if config.dump_parse_tree {
        print_tree(&prepared);
    }
    Ok((file, prepared, text))
}

pub fn doc_graph_to_dot(config: &Config, code: &str) -> Result<String> {
    let (_, file, _) = prepare_source_code(config, code)?;
    let fmt = Formatter::new(config);
    let mut chunks = Vec::new();
    format_source_file(&fmt, &file, |chunk| chunks.push(chunk));
    Ok(fmt.doc_graph_to_dot(&Builder::concat(&fmt, chunks)))
}

pub struct Formatted {
    pub code: String,
    /// The source copied unchanged because afmt cannot format it yet (with
    /// `Config::allow_unsupported`)
    pub unsupported: Vec<UnsupportedSyntax>,
}

/// In source order; a node in more layouts is listed once
fn locate_unsupported(source: &str, mut unsupported: Vec<Unsupported>) -> Vec<UnsupportedSyntax> {
    unsupported.sort_by_key(|u| (u.range.start(), u.range.end()));
    unsupported.dedup_by_key(|u| u.range);
    let mut line_columns = LineColumns::new(source);
    unsupported
        .into_iter()
        .map(|u| {
            let (line, column) = line_columns.at(u.range.start());
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

/// `\r\n` is read as `\n`, as rustc does (also in comments and literals); the output has the line
/// endings of `newline_style`
pub fn format_source_code(config: &Config, code: &str) -> Result<Formatted> {
    let source = code;
    let code = &*code.replace("\r\n", "\n");
    let (file, prepared, prepared_text) = prepare_source_code(config, code)?;

    let fmt = Formatter::new(config);
    let mut out = String::with_capacity(code.len() * 2);
    // Each chunk is rendered as soon as it is built, so the arenas hold one top-level item at a time
    format_source_file(&fmt, &prepared, |chunk| {
        if config.dump_doc_tree {
            fmt.dump(&chunk);
        }
        fmt.render_into(chunk, &mut out);
    });
    let unsupported = locate_unsupported(&prepared_text, fmt.take_unsupported());
    if !unsupported.is_empty() && !config.allow_unsupported {
        return Err(Error::Unsupported(unsupported));
    }

    if config.check_result {
        let out_parse = SourceFile::parse(&out, config.edition);
        let errors = out_parse.errors();
        if !errors.is_empty() {
            return Err(Error::GenericError(format!(
                "Could not reparse the result: {:?}",
                errors
            )));
        }
        if let Err(error) = check_result(&file, &out_parse.tree(), code, &out, config.edition) {
            return Err(Error::GenericError(error));
        }
    }

    if config.newline_style.uses_crlf(source) {
        out = out.replace('\n', "\r\n");
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

    fn with_newlines(style: NewlineStyle, code: &str) -> String {
        let config = Config {
            newline_style: style,
            ..Config::default()
        };
        format_source_code(&config, code).unwrap().code
    }

    #[test]
    fn crlf_is_read_as_lf_and_written_by_newline_style() {
        // A comment and a block comment, whose text would keep a `\r`, and a string literal
        let lf = "fn f() {\n    // c\n    let x = 1; /* a\n b */\n    \"s\nt\"\n}";
        let crlf = lf.replace('\n', "\r\n");
        let crlf_input = crlf.replace("let x", "let  x") + "\r\n";
        assert_eq!(with_newlines(NewlineStyle::Unix, &crlf_input), lf);
        assert_eq!(with_newlines(NewlineStyle::Auto, &crlf_input), crlf);
        assert_eq!(with_newlines(NewlineStyle::Auto, lf), lf);
        assert_eq!(with_newlines(NewlineStyle::Windows, lf), crlf);
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
    fn normalize_comment_sigil_spacing() {
        // empty comments (`/**/`, `//`) get no space
        let code = "fn/*a*/f() { g(/**/ 1 /*  b  */, /** c*/ 2); //c\n //\n}";
        // off by default, as in rustfmt
        let config = Config::default();
        assert_eq!(
            format_source_code(&config, code).unwrap().code,
            "fn /*a*/ f() {\n    g(/**/ 1 /*  b  */, /** c*/ 2); //c\n    //\n}"
        );
        let config = Config {
            normalize_comment_sigil_spacing: true,
            ..Config::default()
        };
        assert_eq!(
            format_source_code(&config, code).unwrap().code,
            "fn /* a */ f() {\n    g(/**/ 1 /* b */, /** c*/ 2); // c\n    //\n}"
        );
    }
}
