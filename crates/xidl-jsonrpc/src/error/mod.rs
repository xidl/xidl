#[cfg(test)]
mod test;

use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Clone, Copy)]
pub enum ErrorCode {
    ParseError,
    InvalidRequest,
    MethodNotFound,
    InvalidParams,
    InternalError,
    ServerError,
    /// A JSON-RPC error code outside the reserved range.
    Custom(i64),
}

impl ErrorCode {
    pub fn code(self) -> i64 {
        match self {
            Self::ParseError => -32700,
            Self::InvalidRequest => -32600,
            Self::MethodNotFound => -32601,
            Self::InvalidParams => -32602,
            Self::InternalError => -32603,
            Self::ServerError => -32000,
            Self::Custom(code) => code,
        }
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.code())
    }
}

/// Maps a raw JSON-RPC error code onto the closest known variant.
impl From<i64> for ErrorCode {
    fn from(code: i64) -> Self {
        match code {
            -32700 => Self::ParseError,
            -32600 => Self::InvalidRequest,
            -32601 => Self::MethodNotFound,
            -32602 => Self::InvalidParams,
            -32603 => Self::InternalError,
            -32000 => Self::ServerError,
            other => Self::Custom(other),
        }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[cfg(feature = "msgpack")]
    #[error("msgpack error: {0}")]
    Msgpack(String),
    /// Indicates a wire frame exceeded the configured maximum length.
    #[error("frame exceeds maximum length {max} bytes ({framing})")]
    FrameTooLarge { max: usize, framing: &'static str },
    #[error("rpc error {code}: {message}")]
    Rpc {
        code: ErrorCode,
        message: String,
        data: Option<Value>,
    },
    #[error("protocol error: {0}")]
    Protocol(&'static str),
}

impl Error {
    pub fn method_not_found(method: &str) -> Self {
        Self::Rpc {
            code: ErrorCode::MethodNotFound,
            message: format!("method not found: {method}"),
            data: None,
        }
    }

    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::Rpc {
            code: ErrorCode::InvalidParams,
            message: message.into(),
            data: None,
        }
    }

    pub fn is_method_not_found(&self) -> bool {
        matches!(
            self,
            Error::Rpc {
                code: ErrorCode::MethodNotFound,
                ..
            }
        )
    }
}
