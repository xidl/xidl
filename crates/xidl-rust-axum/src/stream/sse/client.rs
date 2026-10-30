use super::{
    Reader, StreamException,
    decoder::{SseDecodeState, StreamAction},
};
use crate::{
    Error,
    stream::{StreamError, StreamFailure},
};
use futures_util::{StreamExt, TryStreamExt};
use reqwest::Request;
use serde::de::DeserializeOwned;
use tokio_util::{
    codec::{FramedRead, LinesCodec},
    io::StreamReader,
};

/// Opens an SSE response using the operation's exception decoder.
pub async fn open_sse<T, E>(
    http: &reqwest::Client,
    req: Request,
) -> std::result::Result<Reader<T, E>, E>
where
    T: DeserializeOwned + Send + 'static,
    E: StreamException + From<Error> + Send + 'static,
{
    let response = http
        .execute(req)
        .await
        .map_err(|error| E::from(Error::message(format!("SSE request: {error}"))))?;
    if !response.status().is_success() {
        let status = response.status().as_u16();
        let bytes = response
            .bytes()
            .await
            .map_err(|error| E::from(Error::message(format!("SSE HTTP error body: {error}"))))?;
        let body = serde_json::from_slice(&bytes).unwrap_or_else(|_| {
            serde_json::Value::String(String::from_utf8_lossy(&bytes).into_owned())
        });
        return Err(E::from_failure(StreamFailure { status, body }).map_err(E::from)?);
    }
    let bytes = response
        .bytes_stream()
        .map_err(|error| std::io::Error::other(error.to_string()));
    let lines = FramedRead::new(StreamReader::new(bytes), LinesCodec::new());
    let stream = futures_util::stream::unfold(
        Some((lines, SseDecodeState::default())),
        |state| async move {
            let (mut lines, mut decoder) = state?;
            loop {
                let action = match lines.next().await {
                    Some(Ok(line)) => decoder.push_line::<T, E>(&line),
                    Some(Err(error)) => Err(Error::message(format!("SSE transport: {error}"))),
                    None => Err(Error::message("SSE EOF before a terminal frame")),
                };
                match action {
                    Ok(StreamAction::Continue) => {}
                    Ok(StreamAction::Item(item)) => {
                        return Some((Ok(item), Some((lines, decoder))));
                    }
                    Ok(StreamAction::Failed(error)) => {
                        return Some((Err(StreamError::Failed(error)), None));
                    }
                    Ok(StreamAction::Done) => return None,
                    Err(error) => return Some((Err(StreamError::Interrupted(error)), None)),
                }
            }
        },
    );
    Ok(Reader::new(Box::pin(stream)))
}
