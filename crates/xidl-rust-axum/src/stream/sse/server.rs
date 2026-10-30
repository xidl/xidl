use super::{SseStream, StreamException};
use crate::Error;
use axum::response::{IntoResponse, Sse, sse::Event};
use futures_util::StreamExt;
use serde::Serialize;
use std::convert::Infallible;

struct SseEncoder;

impl SseEncoder {
    fn next<T: Serialize>(value: T) -> Result<Event, Error> {
        let json = serde_json::to_string(&value)
            .map_err(|error| Error::message(format!("SSE item serialization: {error}")))?;
        Ok(Event::default().event("next").data(json))
    }

    fn failure<E: StreamException>(error: E) -> Event {
        let encoded = error.into_failure().and_then(|failure| {
            serde_json::to_string(&failure)
                .map_err(|error| Error::message(format!("SSE exception serialization: {error}")))
        });
        let json = match encoded {
            Ok(json) => json,
            Err(_) => {
                r#"{"status":500,"body":{"code":500,"msg":"SSE exception serialization failed"}}"#
                    .into()
            }
        };
        Event::default().event("error").data(json)
    }

    async fn response<T, E>(mut source: SseStream<T, E>) -> axum::response::Response
    where
        T: Serialize + 'static,
        E: StreamException + IntoResponse + 'static,
    {
        let first = match source.next().await {
            Some(Ok(item)) => match Self::next(item) {
                Ok(event) => (event, true),
                Err(error) => return error.into_response(),
            },
            Some(Err(error)) => return error.into_response(),
            None => (Event::default().event("complete").data("{}"), false),
        };
        let frames =
            futures_util::stream::unfold(Some((Some(first), source)), |state| async move {
                let (first, mut source) = state?;
                if let Some((event, more)) = first {
                    return Some((Ok::<_, Infallible>(event), more.then_some((None, source))));
                }
                let (event, terminal) = match source.next().await {
                    Some(Ok(item)) => match Self::next(item) {
                        Ok(event) => (event, false),
                        Err(error) => (Self::failure(error), true),
                    },
                    Some(Err(error)) => (Self::failure(error), true),
                    None => (Event::default().event("complete").data("{}"), true),
                };
                Some((Ok(event), (!terminal).then_some((None, source))))
            });
        Sse::new(frames).into_response()
    }
}

/// Prepares the first output before committing the SSE response.
pub async fn sse_response<T, E>(source: SseStream<T, E>) -> axum::response::Response
where
    T: Serialize + 'static,
    E: StreamException + IntoResponse + 'static,
{
    SseEncoder::response(source).await
}
