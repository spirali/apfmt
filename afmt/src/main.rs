use std::io::Read;
use std::io::{Write};
use std::path::PathBuf;

use afmt_core::Config;
use clap::Parser;

#[derive(Parser)]
#[command(version, about = "TODO")]
struct Cli {
    /// Path to the file to parse; reads from stdin if not provided
    file: Option<PathBuf>,

    /// Dump parse tree
    #[clap(long)]
    dump_tree: bool,

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

fn main() {
    let cli = Cli::parse();
    let code = match &cli.file {
        Some(path) => std::fs::read_to_string(path).unwrap(),
        None => {
            let mut code = String::new();
            std::io::stdin().read_to_string(&mut code).unwrap();
            code
        }
    };
    let cfg = Config {
        dump_tree: cli.dump_tree,
        check_result: cli.result_check,
        ..Config::default()
    };
    let out = match afmt_core::format_source_code(&cfg, &code) {
        Ok(out) => out,
        Err(e) => {
            process_error(e);
            std::process::exit(1);
        }
    };
    println!("{}", out);
}
