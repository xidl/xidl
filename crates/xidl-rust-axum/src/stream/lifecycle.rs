use crate::{Error, ErrorBody, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Observable terminal outcome of a structured HTTP stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamOutcome {
    /// A complete frame was received.
    Completed,
    /// An error frame was received.
    Failed,
    /// The consumer explicitly cancelled the stream.
    Cancelled,
    /// The transport ended without a terminal frame.
    Interrupted,
}

/// Error payload shared by HTTP responses and terminal stream frames.
#[derive(Debug, Serialize, Deserialize)]
pub struct StreamFailure {
    /// HTTP status of the projected exception.
    pub status: u16,
    /// JSON body of the projected exception.
    #[serde(deserialize_with = "Value::deserialize")]
    pub body: Value,
}

impl StreamFailure {
    pub(super) fn validate(&self) -> Result<()> {
        if !(300..=599).contains(&self.status) {
            return Err(Error::message(
                "stream failure status must be 300 through 599",
            ));
        }
        Ok(())
    }
}

/// Operation-specific conversion between declared exceptions and wire failures.
pub trait StreamException: Sized {
    /// Encodes the exception without selecting or inventing a business error.
    fn into_failure(self) -> Result<StreamFailure>;
    /// Decodes the failure against the operation's projected exception set.
    fn from_failure(failure: StreamFailure) -> Result<Self>;
}

impl StreamException for Error {
    fn into_failure(self) -> Result<StreamFailure> {
        let status = self.http_status().as_u16();
        let body = serde_json::to_value(ErrorBody::from(self))
            .map_err(|error| Error::message(format!("stream error encoding: {error}")))?;
        Ok(StreamFailure { status, body })
    }

    fn from_failure(failure: StreamFailure) -> Result<Self> {
        let status = failure.status;
        let message = match serde_json::from_value::<ErrorBody>(failure.body.clone()) {
            Ok(body) => body.msg,
            Err(_) => format!("undeclared stream error: {}", failure.body).into(),
        };
        Ok(Self::new(status, message))
    }
}

/// A declared stream failure or an interrupted transport.
#[derive(Debug, thiserror::Error)]
pub enum StreamError<E = Error> {
    /// The producer terminated with a declared or runtime exception.
    #[error("{0}")]
    Failed(E),
    /// No valid terminal frame was received.
    #[error("stream interrupted: {0}")]
    Interrupted(Error),
}

impl<E> StreamError<E> {
    pub(super) fn outcome(&self) -> StreamOutcome {
        match self {
            Self::Failed(_) => StreamOutcome::Failed,
            Self::Interrupted(_) => StreamOutcome::Interrupted,
        }
    }
}
