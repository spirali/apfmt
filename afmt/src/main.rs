use std::io::Read;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use afmt_core::{read_config, Config, Edition, UnsupportedSyntax};
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
#[command(version, about = "Rust code formatter")]
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

    /// The edition of the syntax (2015, 2018, 2021 or 2024); overrides the edition of the package in
    /// the nearest `Cargo.toml` (as `cargo fmt` does), which overrides `edition` in the rustfmt config
    /// file [default: 2024]
    #[clap(long, value_parser = Edition::from_str)]
    edition: Option<Edition>,

    /// The rustfmt config like file to use, or a directory with:
    /// `rustfmt.toml`, `.rustfmt.toml`, `afmt.toml`, `.afmt.toml`, instead
    /// of looking it up from the directory of the file (or the current directory for stdin), its
    /// ancestors, the home directory and the user's configuration directory, like rustfmt
    #[clap(long, conflicts_with = "no_config")]
    config_path: Option<PathBuf>,

    /// Do not read any rustfmt config file, nor the edition from `Cargo.toml`
    #[clap(long)]
    no_config: bool,

    /// Do not ready any rustfmt config file
    #[clap(long)]
    no_rustfmt_toml: bool,
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
        afmt_core::Error::IoError(error) => {
            writeln!(handle, "{}", error).unwrap();
        }
    };
}

fn exit_with_io_error(what: &str, error: std::io::Error) -> ! {
    eprintln!("afmt: {what}: {error}");
    std::process::exit(1);
}

fn config_file(cli: &Cli) -> Option<PathBuf> {
    if let Some(path) = &cli.config_path {
        let file = if path.is_dir() {
            afmt_core::config_in_dir(path, true, !cli.no_rustfmt_toml)
        } else {
            path.exists().then(|| path.clone())
        };
        if file.is_none() {
            eprintln!("afmt: no config file found at {}", path.display());
            std::process::exit(1);
        }
        return file;
    }
    afmt_core::find_config(source_dir(cli), !cli.no_rustfmt_toml)
}

/// The directory where the config files of the source are looked up from: the directory of the file,
/// or the current directory for stdin
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

/// Sets the edition of the package in the nearest `Cargo.toml`, if there is one with `[package]`;
/// exits with code 1 when it cannot be read
fn try_apply_cargo_edition(cli: &Cli, cfg: &mut Config) {
    let Some(manifest) = afmt_core::find_cargo_manifest(source_dir(cli)) else {
        return;
    };
    match afmt_core::cargo_edition(&manifest) {
        Ok(Some(edition)) => cfg.edition = edition,
        Ok(None) => {}
        Err(afmt_core::Error::GenericError(message)) => {
            eprintln!("afmt: cannot read the edition: {message}");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("afmt: cannot read the edition from {}: {e:?}", manifest.display());
            std::process::exit(1);
        }
    }
}

fn try_apply_config(cli: &Cli, cfg: &mut Config) {
    if let Some(config_path) = config_file(cli) {
        match read_config(&config_path, cfg) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("cannot read config file: {e:?}");
                std::process::exit(1);
            }
        };
    }
}

/// Writes `out` to stdout; a closed stdout (e.g. `afmt file.rs | head`) is not an error
fn write_output(out: &str) {
    // Not `print!`, which panics when stdout is closed
    if let Err(error) = std::io::stdout().write_all(out.as_bytes())
        && error.kind() != std::io::ErrorKind::BrokenPipe
    {
        exit_with_io_error("cannot write the output", error);
    }
}

fn main() {
    let cli = Cli::parse();

    let mut cfg = Config::default();
    if !cli.no_config {
        try_apply_config(&cli, &mut cfg);
        try_apply_cargo_edition(&cli, &mut cfg);
    }
    cfg.dump_parse_tree = cli.dump_parse_tree;
    cfg.dump_doc_tree = cli.dump_doc_tree;
    cfg.check_result = cli.result_check;
    cfg.allow_unsupported = cli.unsupported != Unsupported::Error;

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

    if let Some(edition) = cli.edition {
        cfg.edition = edition;
    }

    if cfg.disable_all_formatting {
        write_output(&code);
        return;
    }
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
    write_output(&out);
}
