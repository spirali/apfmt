//! Terminal output: when it is coloured (`--color`), and cargo-style status lines

use anstyle::{AnsiColor, Style};
use clap::ValueEnum;

/// When the output is coloured
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum ColorWhen {
    /// When the output is a terminal, unless `NO_COLOR` is set (or with `CLICOLOR_FORCE`)
    Auto,
    Always,
    Never,
}

/// Sets when the output is coloured: `when` (`--color`), else `CARGO_TERM_COLOR` as in cargo, else
/// `ColorWhen::Auto`
pub fn set_color(when: Option<ColorWhen>) {
    let from_env = || {
        let value = std::env::var("CARGO_TERM_COLOR").ok()?;
        ColorWhen::from_str(&value, true).ok()
    };
    let choice = match when.or_else(from_env) {
        Some(ColorWhen::Always) => anstream::ColorChoice::Always,
        Some(ColorWhen::Never) => anstream::ColorChoice::Never,
        Some(ColorWhen::Auto) | None => anstream::ColorChoice::Auto,
    };
    choice.write_global();
}

const fn bold(color: AnsiColor) -> Style {
    Style::new()
        .bold()
        .fg_color(Some(anstyle::Color::Ansi(color)))
}

pub const SUCCESS: Style = bold(AnsiColor::Green);
pub const FAILURE: Style = bold(AnsiColor::Red);
pub const WARNING: Style = bold(AnsiColor::Yellow);
pub const DIM: Style = Style::new().dimmed();

/// The width of the verb of a status line, as in cargo
const VERB_WIDTH: usize = 12;

/// A status line on stderr, as cargo prints them: `verb` right-aligned in `style`, then `text`
pub fn status(verb: &str, style: Style, text: &str) {
    anstream::eprintln!("{style}{verb:>VERB_WIDTH$}{style:#} {text}");
}

/// A line on stderr under a status line, aligned with its text and dimmed
pub fn status_detail(text: &str) {
    anstream::eprintln!("{:VERB_WIDTH$} {DIM}{text}{DIM:#}", "");
}

/// A warning on stderr: `warning: message`
pub fn warning(message: &str) {
    anstream::eprintln!("{WARNING}warning{WARNING:#}: {message}");
}

/// An error that stops `program`, on stderr: `program: message`
pub fn error(program: &str, message: &str) {
    anstream::eprintln!("{FAILURE}{program}:{FAILURE:#} {message}");
}

/// `text` in `style`, when the output is coloured
pub fn styled(style: Style, text: &str) -> String {
    format!("{style}{text}{style:#}")
}
