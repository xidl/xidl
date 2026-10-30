mod bidi;
mod bytes;
mod lifecycle;
mod ndjson;
mod sse;
mod writer;

pub use bidi::BidiClientStream;
#[cfg(feature = "server")]
pub use bidi::BidiServerStream;
#[cfg(all(not(tarpaulin_include), feature = "server"))]
pub use bidi::open_bidi_server;
#[cfg(not(tarpaulin_include))]
pub use bidi::{open_bidi_client, open_bidi_client_with_headers};
#[cfg(feature = "client")]
pub use bytes::open_byte_stream;
pub use bytes::{ByteReader, boxed_bytes};
#[cfg(feature = "server")]
pub use bytes::{byte_stream_response, decode_bytes_body};
pub use lifecycle::{StreamError, StreamException, StreamFailure, StreamOutcome};
pub use ndjson::boxed_ndjson;
#[cfg(feature = "server")]
pub use ndjson::decode_ndjson_body;
#[cfg(feature = "client")]
pub use ndjson::encode_ndjson_body;
#[cfg(feature = "client")]
pub use sse::open_sse;
#[cfg(feature = "server")]
pub use sse::sse_response;
pub use sse::{Reader, boxed_sse};
pub use writer::ClientStreamWriter;

use crate::Result;
use futures_util::Stream;
#[cfg(test)]
use ndjson::decode_ndjson_reader;
#[cfg(all(test, feature = "client"))]
use sse::{SseDecodeState, StreamAction};
use std::pin::Pin;

/// Boxed raw byte stream used for octet sequence.
pub type ByteStream = Pin<Box<dyn Stream<Item = Result<::bytes::Bytes>> + Send + 'static>>;

/// Boxed server-sent event stream used by generated handlers.
pub type SseStream<T, E = crate::Error> =
    Pin<Box<dyn Stream<Item = std::result::Result<T, E>> + Send + 'static>>;
/// Boxed server-sent event stream consumed by generated clients.
pub type SseClientStream<T, E = crate::Error> =
    Pin<Box<dyn Stream<Item = std::result::Result<T, StreamError<E>>> + Send + 'static>>;
/// Boxed NDJSON receive stream.
pub type NdjsonStream<T> = Pin<Box<dyn Stream<Item = Result<T>> + Send + 'static>>;
/// Boxed NDJSON send stream.
pub type NdjsonSendStream<T> = Pin<Box<dyn Stream<Item = Result<T>> + Send + 'static>>;
/// Alias for a client-stream writer.
pub type Writer<T, R, E = crate::Error> = ClientStreamWriter<T, R, E>;
/// Alias for a bidirectional client stream.
pub type ReaderWriter<TIn, TOut> = BidiClientStream<TIn, TOut>;

#[cfg(test)]
mod tests;
