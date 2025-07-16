use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum ParseError {
    #[error("{0}")]
    Message(String),
    #[error("{0}")]
    Utf8Error(#[from] std::str::Utf8Error),
}

pub type ParserResult<T> = Result<T, ParseError>;
