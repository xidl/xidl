#[cfg(test)]
mod test;

use serde_json::Value;

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

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Json(serde_json::Error),
    #[cfg(feature = "msgpack")]
    Msgpack(String),
    /// Indicates a wire frame exceeded the configured maximum length.
    FrameTooLarge {
        max: usize,
        framing: &'static str,
    },
    Rpc {
        code: ErrorCode,
        message: String,
        data: Option<Value>,
    },
    /// A request did not receive a response before its deadline.
    RequestTimeout,
    Protocol(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "io error: {err}"),
            Self::Json(err) => write!(f, "json error: {err}"),
            #[cfg(feature = "msgpack")]
            Self::Msgpack(message) => write!(f, "msgpack error: {message}"),
            Self::FrameTooLarge { max, framing } => {
                write!(f, "frame exceeds maximum length {max} bytes ({framing})")
            }
            Self::Rpc { code, message, .. } => write!(f, "rpc error {code}: {message}"),
            Self::RequestTimeout => f.write_str("rpc request timed out"),
            Self::Protocol(message) => write!(f, "protocol error: {message}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Json(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
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
