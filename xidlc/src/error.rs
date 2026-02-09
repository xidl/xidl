use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IdlcError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Parse(#[from] xidl_parser::error::ParseError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Template(String),
    #[error("{0}")]
    Rpc(String),
    #[error("{0}")]
    Fmt(String),
    #[error("{0}")]
    Diagnostic(#[from] DiagnosticError),
}

#[derive(Error, Debug, Diagnostic)]
#[error("Parse source error:")]
#[diagnostic()]
pub struct DiagnosticError {
    // The Source that we're gonna be printing snippets out of.
    // This can be a String if you don't have or care about file names.
    #[source_code]
    pub src: NamedSource<String>,
    // Snippets and highlights can be included in the diagnostic!
    #[label("Error here")]
    pub bad_bit: SourceSpan,
}

pub type IdlcResult<T> = std::result::Result<T, IdlcError>;

impl IdlcError {
    pub fn template(message: impl Into<String>) -> Self {
        Self::Template(message.into())
    }

    pub fn rpc(message: impl Into<String>) -> Self {
        Self::Rpc(message.into())
    }

    pub fn fmt(message: impl Into<String>) -> Self {
        Self::Fmt(message.into())
    }

    pub fn diagnostic(err: DiagnosticError) -> Self {
        Self::Diagnostic(err)
    }
}
