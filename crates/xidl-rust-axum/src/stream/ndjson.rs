use super::{NdjsonSendStream, NdjsonStream};
use crate::{Error, Result};
use axum::body::Body;
use futures_util::{Stream, StreamExt, TryStreamExt};
#[cfg(feature = "client")]
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::io::AsyncRead;
use tokio_util::codec::{FramedRead, LinesCodec};
use tokio_util::io::StreamReader;

/// Boxes an NDJSON stream for runtime consumption.
pub fn boxed_ndjson<T, S>(stream: S) -> NdjsonStream<T>
where
    S: Stream<Item = Result<T>> + Send + 'static,
{
    Box::pin(stream)
}

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
    let mapped = stream.map(|item| match item {
        Ok(value) => {
            let mut line =
                serde_json::to_vec(&value).map_err(|err| std::io::Error::other(err.to_string()))?;
            line.push(b'\n');
            Ok::<Vec<u8>, std::io::Error>(line)
        }
        Err(err) => Err(std::io::Error::other(err.to_string())),
    });
    reqwest::Body::wrap_stream(mapped)
}

pub(super) fn decode_ndjson_reader<T, R>(reader: R) -> NdjsonStream<T>
where
    T: DeserializeOwned + Send + 'static,
    R: AsyncRead + Unpin + Send + 'static,
{
    let lines = FramedRead::new(reader, LinesCodec::new());
    let out = lines.map(|line| match line {
        Ok(line) => serde_json::from_str::<T>(&line)
            .map_err(|err| Error::new(400, format!("invalid ndjson payload: {err}"))),
        Err(err) => Err(Error::new(500, err.to_string())),
    });
    Box::pin(out)
}
