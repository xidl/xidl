use super::ByteStream;
use crate::{Error, Result};
use axum::body::{Body, Bytes};
use futures_util::{Stream, StreamExt};
#[cfg(feature = "client")]
use reqwest::Request;

/// Boxes a raw byte stream for server-side response.
pub fn boxed_bytes<S>(stream: S) -> ByteStream
where
    S: Stream<Item = Result<Bytes>> + Send + 'static,
{
    Box::pin(stream)
}

/// Decodes an Axum request body as a raw byte stream (client → server upload).
pub fn decode_bytes_body(body: Body) -> ByteStream {
    let stream = body
        .into_data_stream()
        .map(|r| r.map_err(|e| Error::new(500, e.to_string())));
    Box::pin(stream)
}

/// Converts a raw byte stream into an HTTP response (server → client download).
pub fn byte_stream_response(stream: ByteStream) -> axum::response::Response {
    use axum::response::IntoResponse;
    let body = Body::from_stream(
        stream.map(|r: Result<Bytes>| r.map_err(|e: Error| std::io::Error::other(e.to_string()))),
    );
    (
        [(axum::http::header::CONTENT_TYPE, "application/octet-stream")],
        body,
    )
        .into_response()
}

/// Client-side reader for raw byte streams (server → client download).
pub struct ByteReader {
    inner: ByteStream,
}

impl ByteReader {
    /// Wraps a boxed byte stream.
    pub fn new(inner: ByteStream) -> Self {
        Self { inner }
    }

    /// Reads the next chunk from the stream.
    pub async fn read(&mut self) -> Option<Result<Bytes>> {
        self.inner.next().await
    }
}

#[cfg(feature = "client")]
/// Opens a raw byte stream response from an HTTP GET (server → client download).
pub async fn open_byte_stream(http: &reqwest::Client, req: Request) -> Result<ByteReader> {
    use crate::ErrorBody;
    let resp = http
        .execute(req)
        .await
        .map_err(|e| Error::new(500, e.to_string()))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.json::<ErrorBody>().await.ok();
        return Err(Error::from_http_response(status, body));
    }
    let stream = resp
        .bytes_stream()
        .map(|r| r.map_err(|e| Error::new(500, e.to_string())));
    Ok(ByteReader::new(Box::pin(stream)))
}
