use axum::response::{IntoResponse, Response};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Error)]
#[error("{message}")]
pub struct Error {
    pub code: u16,
    pub message: String,
}

impl Error {
    pub fn new(code: u16, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn message(message: impl Into<String>) -> Self {
        Self::new(500, message)
    }
}

impl From<Error> for ErrorBody {
    fn from(err: Error) -> Self {
        Self {
            code: err.code,
            msg: err.message,
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let body: ErrorBody = self.into();
        (StatusCode::from_u16(body.code).unwrap(), axum::Json(body)).into_response()
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBody {
    pub code: u16,
    pub msg: String,
}
