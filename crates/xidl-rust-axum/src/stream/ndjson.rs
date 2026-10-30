#[cfg(feature = "client")]
use super::NdjsonSendStream;
use super::{NdjsonStream, StreamException};
use crate::{Error, Result};
#[cfg(feature = "server")]
use axum::body::Body;
#[cfg(feature = "server")]
use futures_util::TryStreamExt;
use futures_util::{Stream, StreamExt};
#[cfg(feature = "client")]
use serde::Serialize;
#[cfg(feature = "server")]
use tokio_util::io::StreamReader;
#[cfg(any(feature = "server", test))]
use {
    super::StreamFailure,
    serde::de::DeserializeOwned,
    tokio::io::AsyncRead,
    tokio_util::codec::{FramedRead, LinesCodec},
};

/// Boxes an NDJSON stream for runtime consumption.
pub fn boxed_ndjson<T, S>(stream: S) -> NdjsonStream<T>
where
    S: Stream<Item = Result<T>> + Send + 'static,
{
    Box::pin(stream)
}

#[cfg(feature = "server")]
/// Decodes an Axum request body as an NDJSON stream.
pub fn decode_ndjson_body<T>(body: Body) -> NdjsonStream<T>
where
    T: DeserializeOwned + Send + 'static,
{
    let data_stream = body
        .into_data_stream()
        .map_err(|err| std::io::Error::other(err.to_string()));
    decode_ndjson_reader(StreamReader::new(data_stream))
}

#[cfg(feature = "client")]
/// Encodes an outbound NDJSON stream as a `reqwest` request body.
pub fn encode_ndjson_body<T>(stream: NdjsonSendStream<T>) -> reqwest::Body
where
    T: Serialize + Send + 'static,
{
    let frames = futures_util::stream::unfold(Some((stream, 0u64)), |state| async move {
        let (mut source, seq) = state?;
        let (frame, next) = match source.next().await {
            Some(Ok(value)) => match serde_json::to_value(value) {
                Ok(data) => (
                    serde_json::json!({"t": "next", "seq": seq + 1, "data": data}),
                    Some((source, seq + 1)),
                ),
                Err(error) => {
                    let failure = Error::message(format!("NDJSON item serialization: {error}"))
                        .into_failure();
                    match failure {
                        Ok(error) => (serde_json::json!({"t": "error", "error": error}), None),
                        Err(error) => {
                            return Some((Err(std::io::Error::other(error.to_string())), None));
                        }
                    }
                }
            },
            Some(Err(error)) => match error.into_failure() {
                Ok(error) => (serde_json::json!({"t": "error", "error": error}), None),
                Err(error) => return Some((Err(std::io::Error::other(error.to_string())), None)),
            },
            None => (serde_json::json!({"t": "complete"}), None),
        };
        let line = serde_json::to_vec(&frame)
            .map(|mut line| {
                line.push(b'\n');
                line
            })
            .map_err(std::io::Error::other);
        Some((line, next))
    });
    reqwest::Body::wrap_stream(frames)
}

#[cfg(any(feature = "server", test))]
pub(super) fn decode_ndjson_reader<T, R>(reader: R) -> NdjsonStream<T>
where
    T: DeserializeOwned + Send + 'static,
    R: AsyncRead + Unpin + Send + 'static,
{
    let lines = FramedRead::new(reader, LinesCodec::new());
    let out = futures_util::stream::unfold(Some(lines), |state| async move {
        let mut lines = state?;
        loop {
            let decoded = match lines.next().await {
                Some(Ok(line)) if line.trim().is_empty() => continue,
                Some(Ok(line)) => NdjsonFrame::<T>::decode(&line),
                Some(Err(error)) => Err(Error::new(400, format!("NDJSON transport: {error}"))),
                None => Err(Error::new(
                    400,
                    "NDJSON interrupted before a terminal frame",
                )),
            };
            match decoded {
                Ok(NdjsonFrame::Next(value)) => return Some((Ok(value), Some(lines))),
                Ok(NdjsonFrame::Heartbeat) => {}
                Ok(NdjsonFrame::Complete) => return None,
                Err(error) => return Some((Err(error), None)),
            }
        }
    });
    Box::pin(out.fuse())
}

#[cfg(any(feature = "server", test))]
enum NdjsonFrame<T> {
    Next(T),
    Complete,
    Heartbeat,
}

#[cfg(any(feature = "server", test))]
impl<T> NdjsonFrame<T> {
    fn decode(line: &str) -> Result<Self>
    where
        T: DeserializeOwned,
    {
        let value: serde_json::Value = serde_json::from_str(line)
            .map_err(|error| Error::new(400, format!("NDJSON frame: {error}")))?;
        match value.get("t").and_then(serde_json::Value::as_str) {
            Some("next") => {
                let data = value
                    .get("data")
                    .ok_or_else(|| Error::new(400, "NDJSON next frame requires data"))?;
                serde_json::from_value(data.clone())
                    .map(NdjsonFrame::Next)
                    .map_err(|error| Error::new(400, format!("NDJSON item: {error}")))
            }
            Some("complete") => Ok(NdjsonFrame::Complete),
            Some("heartbeat") => Ok(NdjsonFrame::Heartbeat),
            Some("error") => {
                let failure: StreamFailure = serde_json::from_value(value["error"].clone())
                    .map_err(|error| Error::new(400, format!("NDJSON error frame: {error}")))?;
                failure
                    .validate()
                    .map_err(|error| Error::new(400, format!("NDJSON error frame: {error}")))?;
                Err(Error::from_failure(failure)?)
            }
            _ => Err(Error::new(
                400,
                "NDJSON frame requires next, complete, error or heartbeat",
            )),
        }
    }
}
