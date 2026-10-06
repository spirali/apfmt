use ra_ap_syntax::SyntaxError;

#[derive(Debug)]
pub enum Error {
    SyntaxErrors(Vec<SyntaxError>),
    GenericError(String),
}

pub type Result<T> = std::result::Result<T, Error>;
