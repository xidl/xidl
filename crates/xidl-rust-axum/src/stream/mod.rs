mod bidi;
mod ndjson;
mod sse;
mod writer;

pub use bidi::{BidiClientStream, BidiServerStream};
#[cfg(not(tarpaulin_include))]
pub use bidi::{open_bidi_client, open_bidi_client_with_headers, open_bidi_server};
#[cfg(feature = "client")]
pub use ndjson::encode_ndjson_body;
pub use ndjson::{boxed_ndjson, decode_ndjson_body};
#[cfg(feature = "client")]
pub use sse::open_sse;
pub use sse::{Reader, boxed_sse, sse_response};
pub use writer::ClientStreamWriter;

use crate::Result;
use futures_util::Stream;
#[cfg(test)]
use ndjson::decode_ndjson_reader;
#[cfg(all(test, feature = "client"))]
use sse::{SseDecodeState, StreamAction};
use std::pin::Pin;

/// Boxed server-sent event stream used by generated handlers.
pub type SseStream<T> = Pin<Box<dyn Stream<Item = Result<T>> + Send + 'static>>;
/// Boxed server-sent event stream consumed by generated clients.
pub type SseClientStream<T> = Pin<Box<dyn Stream<Item = Result<T>> + Send + 'static>>;
/// Boxed NDJSON receive stream.
pub type NdjsonStream<T> = Pin<Box<dyn Stream<Item = Result<T>> + Send + 'static>>;
/// Boxed NDJSON send stream.
pub type NdjsonSendStream<T> = Pin<Box<dyn Stream<Item = Result<T>> + Send + 'static>>;
/// Alias for a client-stream writer.
pub type Writer<T, R> = ClientStreamWriter<T, R>;
/// Alias for a bidirectional client stream.
pub type ReaderWriter<TIn, TOut> = BidiClientStream<TIn, TOut>;

#[cfg(test)]
mod tests;
