use ra_ap_syntax::SyntaxError;

#[derive(Debug)]
pub enum Error {
    SyntaxErrors(Vec<SyntaxError>),
    IoError(std::io::Error),
    GenericError(String),
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

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::IoError(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
