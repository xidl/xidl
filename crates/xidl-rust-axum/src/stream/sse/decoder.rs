use super::super::{StreamException, StreamFailure};
use crate::{Error, Result};
use serde::de::DeserializeOwned;

#[derive(Default)]
pub(in crate::stream) struct SseDecodeState {
    event: Option<String>,
    data: Vec<String>,
}

impl SseDecodeState {
    pub(in crate::stream) fn push_line<T: DeserializeOwned, E: StreamException>(
        &mut self,
        line: &str,
    ) -> Result<StreamAction<T, E>> {
        if line.is_empty() {
            return self.flush();
        }
        if line.starts_with(':') {
            return Ok(StreamAction::Continue);
        }
        if let Some(rest) = line.strip_prefix("event:") {
            self.event = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("data:") {
            self.data
                .push(rest.strip_prefix(' ').unwrap_or(rest).to_string());
        }
        Ok(StreamAction::Continue)
    }

    pub(in crate::stream) fn flush<T: DeserializeOwned, E: StreamException>(
        &mut self,
    ) -> Result<StreamAction<T, E>> {
        let event = self.event.take();
        if self.data.is_empty() {
            return Ok(StreamAction::Continue);
        }
        let payload = std::mem::take(&mut self.data).join("\n");
        match event.as_deref().unwrap_or("message") {
            "next" | "message" => serde_json::from_str(&payload)
                .map(StreamAction::Item)
                .map_err(|error| Error::message(format!("invalid SSE item: {error}"))),
            "error" => {
                let failure: StreamFailure = serde_json::from_str(&payload)
                    .map_err(|error| Error::message(format!("invalid SSE error: {error}")))?;
                failure.validate()?;
                E::from_failure(failure).map(StreamAction::Failed)
            }
            "complete" => Ok(StreamAction::Done),
            _ => Ok(StreamAction::Continue),
        }
    }
}

pub(in crate::stream) enum StreamAction<T, E> {
    Continue,
    Item(T),
    Failed(E),
    Done,
}
