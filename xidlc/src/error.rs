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
}
