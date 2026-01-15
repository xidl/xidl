use thiserror::Error;

#[derive(Debug, Error)]
pub enum XcdrError {
    #[error("{0}")]
    Message(String),
    #[error("BufferOverflow")]
    BufferOverflow,
}

pub type XcdrResult<T> = std::result::Result<T, XcdrError>;
