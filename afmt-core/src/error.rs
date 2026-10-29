use ra_ap_syntax::SyntaxError;

#[derive(Debug)]
pub enum Error {
    SyntaxErrors(Vec<SyntaxError>),
    IoError(std::io::Error),
    GenericError(String),
    Unsupported(Vec<UnsupportedSyntax>),
}

/// With `Config::allow_unsupported`, it is copied unchanged into the output
#[derive(Debug, Clone)]
pub struct UnsupportedSyntax {
    /// 1-based, the column in characters, in the source after the import rewrites
    pub line: usize,
    pub column: usize,
    /// What is not supported: a syntax kind (`MACRO_EXPR`) or a description (a `try` block)
    pub what: String,
    pub snippet: String,
}

impl UnsupportedSyntax {
    /// `line:column: not supported yet: what{note}: snippet`
    pub fn message(&self, note: &str) -> String {
        format!(
            "{}:{}: not supported yet: {}{note}: {}",
            self.line, self.column, self.what, self.snippet
        )
    }
}

/// A syntax error per line, the unsupported syntax as `line:column: what: snippet` per line
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::SyntaxErrors(errors) => {
                let errors: Vec<_> = errors.iter().map(|e| format!("{e:?}")).collect();
                write!(f, "{}", errors.join("\n"))
            }
            Error::IoError(error) => write!(f, "{error}"),
            Error::GenericError(message) => write!(f, "{message}"),
            Error::Unsupported(unsupported) => {
                let lines: Vec<_> = unsupported.iter().map(|u| u.message("")).collect();
                write!(f, "{}", lines.join("\n"))
            }
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::IoError(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
