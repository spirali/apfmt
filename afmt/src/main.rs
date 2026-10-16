use std::io::Read;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use afmt::report::ColorWhen;
use afmt::{FormatOptions, error_messages, format_code, report};
use afmt_core::Edition;
use clap::Parser;

#[derive(Parser)]
#[command(version, about = "Rust code formatter")]
struct Cli {
    /// Path to the file to parse; reads from stdin if not provided
    file: Option<PathBuf>,

    /// Dump the output from the parser
    #[arg(long)]
    dump_parse_tree: bool,

    /// Dump doc tree
    #[arg(long)]
    dump_doc_tree: bool,

    /// Print the doc graph in the Graphviz DOT format instead of the formatted code
    #[arg(long)]
    doc_graph_to_dot: bool,

    /// The edition of the syntax (2015, 2018, 2021 or 2024); overrides the edition of the package in
    /// the nearest `Cargo.toml` (as `cargo fmt` does), which overrides `edition` in the config file
    /// [default: 2024]
    #[arg(long, value_parser = Edition::from_str)]
    edition: Option<Edition>,

    /// When to colour the messages [default: auto, or `CARGO_TERM_COLOR`]
    #[arg(long, value_enum, value_name = "WHEN")]
    color: Option<ColorWhen>,

    #[command(flatten)]
    format: FormatOptions,
}

fn exit_with_error(message: &str) -> ! {
    report::error("afmt", message);
    std::process::exit(1);
}

fn source_dir(cli: &Cli) -> &Path {
    let dir = match &cli.file {
        Some(file) => file.parent().unwrap_or(Path::new(".")),
        None => Path::new("."),
    };
    // `Path::parent` of a file name without a directory is ""
    if dir.as_os_str().is_empty() {
        Path::new(".")
    } else {
        dir
    }
}

fn write_output(out: &str) {
    // Not `print!`, which panics when stdout is closed
    if let Err(error) = std::io::stdout().write_all(out.as_bytes())
        && error.kind() != std::io::ErrorKind::BrokenPipe
    {
        exit_with_error(&format!("cannot write the output: {error}"));
    }
}

fn main() {
    let cli = Cli::parse();
    report::set_color(cli.color);

    let mut cfg = cli
        .format
        .config_for_source(source_dir(&cli), cli.edition)
        .unwrap_or_else(|error| exit_with_error(&error.to_string()));
    cfg.dump_parse_tree = cli.dump_parse_tree;
    cfg.dump_doc_tree = cli.dump_doc_tree;

    let code = match &cli.file {
        Some(path) => std::fs::read_to_string(path).unwrap_or_else(|error| {
            exit_with_error(&format!("cannot read {}: {error}", path.display()))
        }),
        None => {
            let mut code = String::new();
            if let Err(error) = std::io::stdin().read_to_string(&mut code) {
                exit_with_error(&format!("cannot read the standard input: {error}"));
            }
            code
        }
    };

    let file = cli
        .file
        .as_ref()
        .map_or("<stdin>".to_string(), |path| path.display().to_string());
    let result = if cli.doc_graph_to_dot {
        afmt_core::doc_graph_to_dot(&cfg, &code).map(|dot| (dot, Vec::new()))
    } else {
        format_code(&cfg, &code, &file, cli.format.unsupported)
    };
    match result {
        Ok((out, warnings)) => {
            for warning in warnings {
                eprintln!("{warning}");
            }
            write_output(&out);
        }
        Err(error) => {
            for message in error_messages(&file, error) {
                eprintln!("{message}");
            }
            std::process::exit(1);
        }
    }
}
