use ra_ap_syntax::SyntaxError;

#[derive(Debug)]
pub enum Error {
    SyntaxErrors(Vec<SyntaxError>),
    GenericError(String),
    /// The source contains syntax that afmt cannot format yet (see `Config::allow_unsupported`)
    Unsupported(Vec<UnsupportedSyntax>),
}

/// Source text that afmt cannot format yet; with `Config::allow_unsupported`, it is copied unchanged
/// into the output
#[derive(Debug, Clone)]
pub struct UnsupportedSyntax {
    /// The line and column (1-based, the column in characters) where the copied text starts, in the
    /// source after the import rewrites (which may move the lines below the imports)
    pub line: usize,
    pub column: usize,
    /// What is not supported: a syntax kind (`MACRO_EXPR`) or a description (a `try` block)
    pub what: String,
    /// The first line of the copied text
    pub snippet: String,
}

pub type Result<T> = std::result::Result<T, Error>;
