use crate::{Error, ErrorBody, Result};
use axum::body::Body;
use axum::response::{IntoResponse, Sse, sse::Event};
use futures_util::stream;
use futures_util::{Stream, StreamExt, TryStreamExt};
use reqwest::Request;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::convert::Infallible;
use std::pin::Pin;
use tokio::io::AsyncRead;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::codec::{FramedRead, LinesCodec};
use tokio_util::io::StreamReader;

pub type SseStream<T> = Pin<Box<dyn Stream<Item = Result<T>> + Send + 'static>>;
pub type SseClientStream<T> = Pin<Box<dyn Stream<Item = Result<T>> + Send + 'static>>;
pub type NdjsonStream<T> = Pin<Box<dyn Stream<Item = Result<T>> + Send + 'static>>;
pub type NdjsonSendStream<T> = Pin<Box<dyn Stream<Item = Result<T>> + Send + 'static>>;

pub struct ClientStreamWriter<T, R> {
    tx: Option<mpsc::Sender<Result<T>>>,
    response: Option<JoinHandle<Result<R>>>,
}

impl<T, R> ClientStreamWriter<T, R> {
    pub fn new(tx: mpsc::Sender<Result<T>>, response: JoinHandle<Result<R>>) -> Self {
        Self {
            tx: Some(tx),
            response: Some(response),
        }
    }

    pub async fn write(&mut self, item: T) -> Result<()> {
        let tx = self
            .tx
            .as_mut()
            .ok_or_else(|| Error::new(500, "stream writer is already closed"))?;
        tx.send(Ok(item))
            .await
            .map_err(|_| Error::new(500, "stream writer is closed"))
    }

    pub async fn close(mut self) -> Result<R> {
        let _ = self.tx.take();
        let response = self
            .response
            .take()
            .ok_or_else(|| Error::new(500, "stream writer is already closed"))?;
        response
            .await
            .map_err(|err| Error::new(500, err.to_string()))?
    }

    pub async fn cancel(mut self) -> Result<()> {
        let _ = self.tx.take();
        if let Some(response) = self.response.take() {
            response.abort();
        }
        Ok(())
    }
}

impl<T, R> Drop for ClientStreamWriter<T, R> {
    fn drop(&mut self) {
        let _ = self.tx.take();
    }
}

pub fn boxed_sse<T, S>(stream: S) -> SseStream<T>
where
    S: Stream<Item = Result<T>> + Send + 'static,
{
    Box::pin(stream)
}

pub fn boxed_ndjson<T, S>(stream: S) -> NdjsonStream<T>
where
    S: Stream<Item = Result<T>> + Send + 'static,
{
    Box::pin(stream)
}

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

pub async fn open_sse<T>(http: &reqwest::Client, req: Request) -> Result<SseClientStream<T>>
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
        let err = match body {
            Some(body) => Error::new(body.code, body.msg),
            None => Error::new(500, format!("http error: {}", status.as_u16())),
        };
        return Err(err);
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

    Ok(Box::pin(out))
}

pub fn decode_ndjson_body<T>(body: Body) -> NdjsonStream<T>
where
    T: DeserializeOwned + Send + 'static,
{
    let data_stream = body
        .into_data_stream()
        .map_err(|err| std::io::Error::other(err.to_string()));
    decode_ndjson_reader(StreamReader::new(data_stream))
}

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

fn decode_ndjson_reader<T, R>(reader: R) -> NdjsonStream<T>
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

#[derive(Default)]
struct SseDecodeState {
    event: Option<String>,
    data: Vec<String>,
}

impl SseDecodeState {
    fn push_line<T: DeserializeOwned>(&mut self, line: &str) -> Result<StreamAction<T>> {
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

    fn flush<T: DeserializeOwned>(&mut self) -> Result<StreamAction<T>> {
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

enum StreamAction<T> {
    Continue,
    Item(T),
    Done,
}
