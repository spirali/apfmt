use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

use afmt_core::Config;
use clap::Parser;

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
}

fn process_error(error: afmt_core::Error) {
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
        ..Config::default()
    };
    let result = if cli.doc_graph_to_dot {
        afmt_core::doc_graph_to_dot(&cfg, &code)
    } else {
        afmt_core::format_source_code(&cfg, &code)
    };
    let out = match result {
        Ok(out) => out,
        Err(e) => {
            process_error(e);
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
