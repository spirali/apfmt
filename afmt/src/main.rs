use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

use afmt_core::{Config, UnsupportedSyntax};
use clap::{Parser, ValueEnum};

/// What to do with syntax that afmt cannot format yet
#[derive(Clone, Copy, PartialEq, ValueEnum)]
enum Unsupported {
    /// Fail and list all of it on stderr, without output
    Error,
    /// Copy it unchanged into the output and list it on stderr
    Warn,
    /// Copy it unchanged into the output
    Allow,
}

#[derive(Parser)]
#[command(version, about = "TODO")]
struct Cli {
    /// Path to the file to parse; reads from stdin if not provided
    file: Option<PathBuf>,

    /// Dump the output from the parser
    #[clap(long)]
    dump_parse_tree: bool,

    /// Dump doc tree
    #[clap(long)]
    dump_doc_tree: bool,

    /// Print the doc graph in the Graphviz DOT format instead of the formatted code
    #[clap(long)]
    doc_graph_to_dot: bool,

    /// Check that only the layout changed: the output must parse to the same syntax tree (apart from
    /// whitespace, comments and trailing commas) and keep all comments; exits with code 1 otherwise
    #[clap(long)]
    result_check: bool,

    /// What to do with syntax that afmt cannot format yet
    #[clap(long, value_enum, default_value_t = Unsupported::Warn)]
    unsupported: Unsupported,
}

/// Lists the syntax that afmt cannot format yet on stderr: `file:line:column: what: snippet`
fn report_unsupported(file: &str, unsupported: &[UnsupportedSyntax], suffix: &str) {
    for u in unsupported {
        eprintln!(
            "{file}:{}:{}: not supported yet: {}{suffix}: {}",
            u.line, u.column, u.what, u.snippet
        );
    }
}

fn process_error(file: &str, error: afmt_core::Error) {
    let stderr = &mut std::io::stderr();
    let mut handle = stderr.lock();
    match error {
        afmt_core::Error::SyntaxErrors(errors) => {
            for error in errors {
                writeln!(handle, "{:?}", error).unwrap();
            }
        }
        afmt_core::Error::GenericError(error) => {
            writeln!(handle, "{}", error).unwrap();
        }
        afmt_core::Error::Unsupported(unsupported) => {
            report_unsupported(file, &unsupported, "");
        }
    };
}

/// Reports an I/O error on stderr and exits with code 1
fn exit_with_io_error(what: &str, error: std::io::Error) -> ! {
    eprintln!("afmt: {what}: {error}");
    std::process::exit(1);
}

fn main() {
    let cli = Cli::parse();
    let code = match &cli.file {
        Some(path) => std::fs::read_to_string(path).unwrap_or_else(|error| {
            exit_with_io_error(&format!("cannot read {}", path.display()), error)
        }),
        None => {
            let mut code = String::new();
            if let Err(error) = std::io::stdin().read_to_string(&mut code) {
                exit_with_io_error("cannot read the standard input", error);
            }
            code
        }
    };
    let cfg = Config {
        dump_parse_tree: cli.dump_parse_tree,
        dump_doc_tree: cli.dump_doc_tree,
        check_result: cli.result_check,
        allow_unsupported: cli.unsupported != Unsupported::Error,
        ..Config::default()
    };
    let file = cli
        .file
        .as_ref()
        .map_or("<stdin>".to_string(), |path| path.display().to_string());
    let result = if cli.doc_graph_to_dot {
        afmt_core::doc_graph_to_dot(&cfg, &code)
    } else {
        afmt_core::format_source_code(&cfg, &code).map(|formatted| {
            if cli.unsupported == Unsupported::Warn {
                report_unsupported(&file, &formatted.unsupported, " (copied unchanged)");
            }
            formatted.code
        })
    };
    let out = match result {
        Ok(out) => out,
        Err(e) => {
            process_error(&file, e);
            std::process::exit(1);
        }
    };
    // An empty file stays empty, otherwise the output ends with a line break
    let out = if out.is_empty() { out } else { out + "\n" };
    // Not `print!`, which panics when stdout is closed (e.g. `afmt file.rs | head`)
    if let Err(error) = std::io::stdout().write_all(out.as_bytes())
        && error.kind() != std::io::ErrorKind::BrokenPipe
    {
        exit_with_io_error("cannot write the output", error);
    }
}
