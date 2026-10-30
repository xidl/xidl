use super::{SseClientStream, SseStream, StreamException, StreamOutcome};
use crate::Error;
use futures_util::{Stream, StreamExt};

#[cfg(feature = "client")]
mod client;
#[cfg(feature = "client")]
mod decoder;
#[cfg(feature = "server")]
mod server;

#[cfg(feature = "client")]
pub use client::open_sse;
#[cfg(all(test, feature = "client"))]
pub(in crate::stream) use decoder::{SseDecodeState, StreamAction};
#[cfg(feature = "server")]
pub use server::sse_response;

/// Client-side reader with an immutable terminal outcome.
pub struct Reader<T, E = Error> {
    inner: Option<SseClientStream<T, E>>,
    outcome: Option<StreamOutcome>,
}

impl<T, E> Reader<T, E> {
    /// Wraps a decoded stream.
    pub fn new(inner: SseClientStream<T, E>) -> Self {
        Self {
            inner: Some(inner),
            outcome: None,
        }
    }

    /// Reads one item; terminal failures are returned once.
    pub async fn read(&mut self) -> Option<std::result::Result<T, super::StreamError<E>>> {
        let item = self.inner.as_mut()?.next().await;
        match &item {
            Some(Ok(_)) => return item,
            Some(Err(error)) => self.outcome = Some(error.outcome()),
            None => self.outcome = Some(StreamOutcome::Completed),
        }
        self.inner = None;
        item
    }

    /// Returns the terminal outcome, or None while the stream is open.
    pub fn outcome(&self) -> Option<StreamOutcome> {
        self.outcome
    }

    /// Drops the response body without changing an existing terminal outcome.
    pub fn cancel(&mut self) {
        if self.outcome.is_none() {
            self.outcome = Some(StreamOutcome::Cancelled);
        }
        self.inner = None;
    }
}

/// Boxes a producer stream while preserving its exception type.
pub fn boxed_sse<T, S, E>(stream: S) -> SseStream<T, E>
where
    S: Stream<Item = std::result::Result<T, E>> + Send + 'static,
{
    Box::pin(stream)
}
