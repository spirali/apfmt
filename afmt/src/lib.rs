//! What `afmt` and `cargo afmt` share: the options of formatting and the messages about the result

pub mod cargo;
pub mod report;

use std::path::{Path, PathBuf};

use afmt_core::{Config, ConfigOptions, Edition, UnsupportedSyntax};
use clap::{Args, ValueEnum};

/// What to do with syntax that afmt cannot format yet
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Unsupported {
    /// Fail and list all of it on stderr, without output
    Error,
    /// Copy it unchanged into the output and list it on stderr
    Warn,
    /// Copy it unchanged into the output
    Allow,
}

/// The options of formatting that both `afmt` and `cargo afmt` have
#[derive(Args, Clone, Debug)]
pub struct FormatOptions {
    /// Check that only the layout changed: the output must parse to the same syntax tree (apart from
    /// whitespace, comments and trailing commas) and keep all comments; exits with code 1 otherwise
    #[arg(long)]
    pub result_check: bool,

    /// What to do with syntax that afmt cannot format yet
    #[arg(long, value_enum, default_value_t = Unsupported::Warn)]
    pub unsupported: Unsupported,

    /// The config file to use, or a directory with one (`.afmt.toml`, `afmt.toml`, `.rustfmt.toml`,
    /// `rustfmt.toml`), instead of looking it up from the directory of the source, its ancestors, the
    /// home directory and the user's configuration directory, like rustfmt
    #[arg(long, conflicts_with = "no_config")]
    pub config_path: Option<PathBuf>,

    /// Read no config file, nor the edition from `Cargo.toml`
    #[arg(long)]
    pub no_config: bool,

    /// Ignore rustfmt's config files (`rustfmt.toml`, `.rustfmt.toml`); `afmt.toml` is still read
    #[arg(long)]
    pub no_rustfmt_toml: bool,
}

impl FormatOptions {
    /// The config for a source in `dir` with these options, and the warnings about it
    pub fn config_for_source(
        &self,
        dir: &Path,
        edition: Option<Edition>,
    ) -> afmt_core::Result<(Config, Vec<String>)> {
        let options = ConfigOptions {
            config_path: self.config_path.clone(),
            no_config: self.no_config,
            no_rustfmt_toml: self.no_rustfmt_toml,
            edition,
        };
        let (mut config, warnings) = afmt_core::config_for_source(dir, &options)?;
        config.check_result = self.result_check;
        config.allow_unsupported = self.unsupported != Unsupported::Error;
        Ok((config, warnings))
    }
}

/// `code` formatted, ending with a line break unless empty, and the warnings about unsupported
/// syntax; with `disable_all_formatting`, `code` unchanged
/// The directory whose config file applies to `file`
pub fn source_dir(file: &Path) -> &Path {
    match file.parent() {
        // `Path::parent` of a file name without a directory is ""
        Some(dir) if !dir.as_os_str().is_empty() => dir,
        _ => Path::new("."),
    }
}

pub fn format_code(
    config: &Config,
    code: &str,
    file: &str,
    unsupported: Unsupported,
) -> afmt_core::Result<(String, Vec<String>)> {
    if config.disable_all_formatting {
        return Ok((code.to_string(), Vec::new()));
    }
    let formatted = afmt_core::format_source_code(config, code)?;
    let warnings = if unsupported == Unsupported::Warn {
        unsupported_messages(file, &formatted.unsupported, " (copied unchanged)")
    } else {
        Vec::new()
    };
    let mut out = formatted.code;
    if !out.is_empty() {
        out.push_str(if config.newline_style.uses_crlf(code) {
            "\r\n"
        } else {
            "\n"
        });
    }
    Ok((out, warnings))
}

/// One message per item: `file:line:column: not supported yet: what{note}: snippet`
pub fn unsupported_messages(
    file: &str,
    unsupported: &[UnsupportedSyntax],
    note: &str,
) -> Vec<String> {
    unsupported
        .iter()
        .map(|u| format!("{file}:{}", u.message(note)))
        .collect()
}

pub fn error_messages(file: &str, error: afmt_core::Error) -> Vec<String> {
    match error {
        afmt_core::Error::Unsupported(unsupported) => unsupported_messages(file, &unsupported, ""),
        afmt_core::Error::SyntaxErrors(errors) => errors
            .iter()
            .map(|error| format!("{file}: {error:?}"))
            .collect(),
        error => vec![format!("{file}: {error}")],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn final_line_break_follows_newline_style() {
        let format = |code| format_code(&Config::default(), code, "a.rs", Unsupported::Allow);
        assert_eq!(format("fn f() {}\r\n").unwrap().0, "fn f() {}\r\n");
        assert_eq!(format("fn f() {}\n").unwrap().0, "fn f() {}\n");
    }
}
