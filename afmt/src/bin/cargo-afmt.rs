//! `cargo afmt`: formats all files of Cargo packages with afmt, like `cargo fmt` does with rustfmt.

use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use afmt::cargo::{Selection, metadata, select_packages};
use afmt::report::{self, ColorWhen, FAILURE, SUCCESS, styled};
use afmt::{FormatOptions, error_messages, format_code};
use afmt_core::{Config, Edition, canonical_path};
use clap::Parser;

#[derive(Parser)]
#[command(
    name = "cargo-afmt",
    bin_name = "cargo afmt",
    version,
    about = "Format all files of Cargo packages with afmt"
)]
struct Cli {
    /// Write nothing; list the files that would change and exit with code 1 if there are any
    #[arg(long)]
    check: bool,

    /// Format only this package (can be repeated); by default the package in the current directory,
    /// or all packages in the workspace root
    #[arg(short = 'p', long = "package", value_name = "PACKAGE")]
    packages: Vec<String>,

    /// Format all packages of the workspace
    #[arg(long, conflicts_with = "packages")]
    all: bool,

    /// The `Cargo.toml` of the package or workspace, instead of the one of the current directory
    #[arg(long, value_name = "PATH")]
    manifest_path: Option<PathBuf>,

    /// Print the targets and the files of each
    #[arg(short, long)]
    verbose: bool,

    /// Print only errors: no warnings, no list of files with `--check` (the exit code tells)
    #[arg(short, long, conflicts_with = "verbose")]
    quiet: bool,

    /// The number of files formatted in parallel [default: the number of CPUs]
    #[arg(short, long, value_name = "N")]
    jobs: Option<NonZeroUsize>,

    /// When to colour the messages [default: auto, or `CARGO_TERM_COLOR`]
    #[arg(long, value_enum, value_name = "WHEN")]
    color: Option<ColorWhen>,

    #[command(flatten)]
    format: FormatOptions,
}

/// What happened to a file
#[derive(Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Unchanged,
    /// Reformatted, or would be with `--check`
    Changed,
    Failed,
}

/// A file of a module tree, read while following the tree
struct SourceFile {
    path: PathBuf,
    /// The edition of its target
    edition: Edition,
    code: String,
}

struct FileResult {
    outcome: Outcome,
    /// Warnings and errors, for stderr
    messages: Vec<String>,
}

/// `path` relative to the current directory when it is inside it, for shorter messages
fn display_path(path: &Path) -> String {
    // Canonical, as the paths of the files are
    static CWD: OnceLock<Option<PathBuf>> = OnceLock::new();
    let cwd = CWD.get_or_init(|| std::env::current_dir().ok().map(|cwd| canonical_path(&cwd)));
    let relative = cwd.as_ref().and_then(|cwd| path.strip_prefix(cwd).ok());
    relative.unwrap_or(path).display().to_string()
}

/// The directory whose config applies to the file at `path`
fn source_dir(path: &Path) -> &Path {
    path.parent().unwrap_or(Path::new("."))
}

/// The config, or the error reading it, for each directory and edition of `files`: the files of a
/// directory share their config file, so it is looked up and read once
fn configs_by_dir<'a>(
    files: &'a [SourceFile],
    options: &FormatOptions,
) -> HashMap<(&'a Path, Edition), Result<Config, String>> {
    let mut configs = HashMap::new();
    for file in files {
        let dir = source_dir(&file.path);
        configs.entry((dir, file.edition)).or_insert_with(|| {
            // The edition of the target, as `cargo fmt` passes it to rustfmt
            options
                .config_for_source(dir, Some(file.edition))
                .map_err(|error| error.to_string())
        });
    }
    configs
}

/// Formats `file` with `config`; with `check`, the file is not written
fn format_file(
    file: &SourceFile,
    config: &Result<Config, String>,
    options: &FormatOptions,
    check: bool,
) -> FileResult {
    let SourceFile { path, code, .. } = file;
    let name = display_path(path);
    let failed = |messages| FileResult {
        outcome: Outcome::Failed,
        messages,
    };
    let config = match config {
        Ok(config) => config,
        Err(error) => return failed(vec![format!("{name}: {error}")]),
    };
    let (out, messages) = match format_code(config, code, &name, options.unsupported) {
        Ok(result) => result,
        Err(error) => return failed(error_messages(&name, error)),
    };
    if out == *code {
        return FileResult {
            outcome: Outcome::Unchanged,
            messages,
        };
    }
    if !check && let Err(error) = std::fs::write(path, out) {
        return failed(vec![format!("{name}: cannot write the file: {error}")]);
    }
    FileResult {
        outcome: Outcome::Changed,
        messages,
    }
}

const THREAD_STACK_SIZE: usize = 64 * 1024 * 1024;

/// `f` of each of `items` on `threads` threads, which take the items in the order of the indices
/// `order`; the results are in the order of `items`
fn parallel_map<T: Sync, R: Send>(
    items: &[T],
    order: &[usize],
    threads: usize,
    f: impl Fn(&T) -> R + Sync,
) -> Vec<R> {
    let next = AtomicUsize::new(0);
    let results = Mutex::new(Vec::with_capacity(items.len()));
    std::thread::scope(|scope| {
        for _ in 0..threads.min(items.len()) {
            let thread = std::thread::Builder::new().stack_size(THREAD_STACK_SIZE);
            let spawned = thread.spawn_scoped(scope, || {
                while let Some(&index) = order.get(next.fetch_add(1, Ordering::Relaxed)) {
                    let result = f(&items[index]);
                    results.lock().unwrap().push((index, result));
                }
            });
            spawned.expect("cannot start a thread");
        }
    });
    let mut results = results.into_inner().unwrap();
    results.sort_by_key(|(index, _)| *index);
    results.into_iter().map(|(_, result)| result).collect()
}

fn exit_with_error(message: &str) -> ! {
    report::error("cargo-afmt", message);
    std::process::exit(1);
}

/// The files of the selected packages, in the order of their paths
fn files_to_format(cli: &Cli) -> Vec<SourceFile> {
    let metadata =
        metadata(cli.manifest_path.as_deref()).unwrap_or_else(|error| exit_with_error(&error));
    let selection = if cli.all {
        Selection::All
    } else if !cli.packages.is_empty() {
        Selection::Named(&cli.packages)
    } else {
        let manifest = cli.manifest_path.clone().or_else(|| {
            std::env::current_dir()
                .ok()
                .and_then(|cwd| afmt_core::find_cargo_manifest(&cwd))
        });
        Selection::Current { manifest }
    };
    let packages =
        select_packages(&metadata, &selection).unwrap_or_else(|error| exit_with_error(&error));
    let mut files = BTreeMap::new();
    for package in packages {
        for target in &package.targets {
            let edition = target
                .edition()
                .unwrap_or_else(|error| exit_with_error(&error));
            let tree = afmt_core::module_files(&target.src_path, edition);
            if !cli.quiet {
                for warning in &tree.warnings {
                    report::warning(warning);
                }
            }
            // The files that no target before has
            let mut new_files = Vec::new();
            for (path, code) in tree.files {
                match files.get(&path) {
                    None => {
                        new_files.push(display_path(&path));
                        files.insert(path, (edition, code));
                    }
                    Some(&(first, _)) if first != edition && !cli.quiet => {
                        report::warning(&format!(
                            "{} is in targets of editions {first} and {edition}; \
                             formatted as {first}",
                            display_path(&path)
                        ));
                    }
                    Some(_) => {}
                }
            }
            if cli.verbose {
                let text = format!(
                    "{} v{} ({} `{}`, edition {edition}): {}",
                    package.name,
                    package.version,
                    target.kind.join(", "),
                    target.name,
                    count(new_files.len(), "file", "files"),
                );
                report::status("Formatting", SUCCESS, &text);
                for file in &new_files {
                    report::status_detail(file);
                }
            }
        }
    }
    files
        .into_iter()
        .map(|(path, (edition, code))| SourceFile {
            path,
            edition,
            code,
        })
        .collect()
}

fn main() {
    // Cargo runs `cargo-afmt afmt <args>`; without cargo, the program is run as `cargo-afmt <args>`
    let mut args: Vec<_> = std::env::args_os().collect();
    if args.get(1).is_some_and(|arg| arg == "afmt") {
        args.remove(1);
    }
    let cli = Cli::parse_from(args);
    report::set_color(cli.color);
    let start = Instant::now();

    let files = files_to_format(&cli);
    let threads = cli
        .jobs
        .or_else(|| std::thread::available_parallelism().ok())
        .map_or(1, NonZeroUsize::get);
    let configs = configs_by_dir(&files, &cli.format);
    // The largest first: a large file started last would keep one thread busy after the others
    let mut largest_first: Vec<_> = (0..files.len()).collect();
    largest_first.sort_by_key(|&index| Reverse(files[index].code.len()));
    let results = parallel_map(&files, &largest_first, threads, |file| {
        let config = &configs[&(source_dir(&file.path), file.edition)];
        format_file(file, config, &cli.format, cli.check)
    });

    let (mut changed, mut failed, mut unchanged) = (0, 0, 0);
    for (SourceFile { path, .. }, result) in files.iter().zip(&results) {
        for message in &result.messages {
            if result.outcome == Outcome::Failed || !cli.quiet {
                eprintln!("{message}");
            }
        }
        match result.outcome {
            Outcome::Failed => failed += 1,
            Outcome::Changed => {
                changed += 1;
                if !cli.quiet && cli.check {
                    // The list for scripts, on stdout
                    println!("{}", display_path(path));
                } else if !cli.quiet {
                    report::status("Reformatted", SUCCESS, &display_path(path));
                }
            }
            Outcome::Unchanged => unchanged += 1,
        }
    }
    if !cli.quiet {
        print_summary(files.len(), [changed, failed, unchanged], cli.check, start);
    }
    if failed > 0 || (cli.check && changed > 0) {
        std::process::exit(1);
    }
}

/// `n` and the noun for it: `1 file`, `2 files`
fn count(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// The last status line: `Finished 3 files in 0.12s: 1 reformatted, 1 failed, 1 unchanged`
fn print_summary(
    files: usize,
    [changed, failed, unchanged]: [usize; 3],
    check: bool,
    start: Instant,
) {
    let changed_text = if check {
        "would be reformatted"
    } else {
        "reformatted"
    };
    let parts: Vec<_> = [
        (changed, changed_text.to_string()),
        (failed, styled(FAILURE, "failed")),
        (unchanged, "unchanged".to_string()),
    ]
    .into_iter()
    .filter(|(n, _)| *n > 0)
    .map(|(n, text)| format!("{n} {text}"))
    .collect();
    let mut text = format!(
        "{} in {:.2}s",
        count(files, "file", "files"),
        start.elapsed().as_secs_f64()
    );
    if !parts.is_empty() {
        text = format!("{text}: {}", parts.join(", "));
    }
    let style = if failed > 0 { FAILURE } else { SUCCESS };
    report::status("Finished", style, &text);
}
