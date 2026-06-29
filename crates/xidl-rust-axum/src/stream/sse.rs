use super::{SseClientStream, SseStream};
#[cfg(feature = "client")]
use crate::Error;
use crate::{ErrorBody, Result};
use axum::response::{IntoResponse, Sse, sse::Event};
#[cfg(feature = "client")]
use futures_util::TryStreamExt;
use futures_util::stream;
use futures_util::{Stream, StreamExt};
#[cfg(feature = "client")]
use reqwest::Request;
use serde::Serialize;
#[cfg(feature = "client")]
use serde::de::DeserializeOwned;
use std::convert::Infallible;
#[cfg(feature = "client")]
use tokio_util::codec::{FramedRead, LinesCodec};
#[cfg(feature = "client")]
use tokio_util::io::StreamReader;

/// Client-side reader for SSE streams.
pub struct Reader<T> {
    inner: SseClientStream<T>,
}

impl<T> Reader<T> {
    /// Wraps a boxed SSE client stream.
    pub fn new(inner: SseClientStream<T>) -> Self {
        Self { inner }
    }

    /// Reads the next item from the stream.
    pub async fn read(&mut self) -> Option<Result<T>> {
        self.inner.next().await
    }
}

/// Boxes a server-sent event stream for runtime consumption.
pub fn boxed_sse<T, S>(stream: S) -> SseStream<T>
where
    S: Stream<Item = Result<T>> + Send + 'static,
{
    Box::pin(stream)
}

/// Converts an item stream into an SSE HTTP response.
pub fn sse_response<T>(stream: SseStream<T>) -> axum::response::Response
where
    T: Serialize + 'static,
{
    let mapped = stream.map(|item| {
        let event = match item {
            Ok(value) => Event::default()
                .event("next")
                .data(serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string())),
            Err(err) => {
                let body: ErrorBody = err.into();
                Event::default().event("error").data(
                    serde_json::to_string(&body)
                        .unwrap_or_else(|_| r#"{"code":500,"msg":"stream error"}"#.to_string()),
                )
            }
        };
        Ok::<_, Infallible>(event)
    });
    let complete = stream::once(async { Ok::<_, Infallible>(Event::default().event("complete")) });
    Sse::new(mapped.chain(complete)).into_response()
}

#[cfg(feature = "client")]
/// Opens an SSE response as a typed client-side reader.
pub async fn open_sse<T>(http: &reqwest::Client, req: Request) -> Result<Reader<T>>
where
    T: DeserializeOwned + Send + 'static,
{
    let resp = http
        .execute(req)
        .await
        .map_err(|err| Error::new(500, err.to_string()))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.json::<ErrorBody>().await.ok();
        return Err(Error::from_http_response(status, body));
    }

    let byte_stream = resp
        .bytes_stream()
        .map_err(|err| std::io::Error::other(err.to_string()));
    let reader = StreamReader::new(byte_stream);
    let lines = FramedRead::new(reader, LinesCodec::new());

    let out = futures_util::stream::try_unfold(
        (lines, SseDecodeState::default()),
        |(mut lines, mut state)| async move {
            loop {
                let Some(line) = lines.next().await else {
                    return Ok(None);
                };
                let line = line.map_err(|err| Error::new(500, err.to_string()))?;
                match state.push_line(&line)? {
                    StreamAction::Continue => {}
                    StreamAction::Item(item) => return Ok(Some((item, (lines, state)))),
                    StreamAction::Done => return Ok(None),
                }
            }
        },
    );

    Ok(Reader::new(Box::pin(out)))
}

#[cfg(feature = "client")]
#[derive(Default)]
pub(super) struct SseDecodeState {
    event: Option<String>,
    data: Vec<String>,
}

#[cfg(feature = "client")]
impl SseDecodeState {
    pub(super) fn push_line<T: DeserializeOwned>(&mut self, line: &str) -> Result<StreamAction<T>> {
        if line.is_empty() {
            return self.flush();
        }
        if line.starts_with(':') {
            return Ok(StreamAction::Continue);
        }
        if let Some(rest) = line.strip_prefix("event:") {
            self.event = Some(rest.trim().to_string());
            return Ok(StreamAction::Continue);
        }
        if let Some(rest) = line.strip_prefix("data:") {
            self.data.push(rest.trim_start().to_string());
            return Ok(StreamAction::Continue);
        }
        Ok(StreamAction::Continue)
    }

    pub(super) fn flush<T: DeserializeOwned>(&mut self) -> Result<StreamAction<T>> {
        if self.event.is_none() && self.data.is_empty() {
            return Ok(StreamAction::Continue);
        }
        let event = self.event.take().unwrap_or_else(|| "message".to_string());
        let payload = self.data.join("\n");
        self.data.clear();

        match event.as_str() {
            "next" | "message" => {
                let value = serde_json::from_str::<T>(&payload)
                    .map_err(|err| Error::new(500, format!("invalid sse payload: {err}")))?;
                Ok(StreamAction::Item(value))
            }
            "error" => {
                if let Ok(body) = serde_json::from_str::<ErrorBody>(&payload) {
                    return Err(Error::new(body.code, body.msg));
                }
                Err(Error::new(500, payload))
            }
            "complete" => Ok(StreamAction::Done),
            _ => Ok(StreamAction::Continue),
        }
    }
}

#[cfg(feature = "client")]
pub(super) enum StreamAction<T> {
    Continue,
    Item(T),
    Done,
}
