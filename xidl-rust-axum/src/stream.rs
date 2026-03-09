use crate::{Error, ErrorBody, Result};
use axum::body::Body;
use axum::extract::ws::{Message as AxumWsMessage, WebSocket as AxumWebSocket};
use axum::response::{IntoResponse, Sse, sse::Event};
use futures_util::stream;
use futures_util::{SinkExt, Stream, StreamExt, TryStreamExt};
use reqwest::Request;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::convert::Infallible;
use std::pin::Pin;
use tokio::io::AsyncRead;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;
use tokio_util::codec::{FramedRead, LinesCodec};
use tokio_util::io::StreamReader;

pub type SseStream<T> = Pin<Box<dyn Stream<Item = Result<T>> + Send + 'static>>;
pub type SseClientStream<T> = Pin<Box<dyn Stream<Item = Result<T>> + Send + 'static>>;
pub type NdjsonStream<T> = Pin<Box<dyn Stream<Item = Result<T>> + Send + 'static>>;
pub type NdjsonSendStream<T> = Pin<Box<dyn Stream<Item = Result<T>> + Send + 'static>>;
pub type Writer<T, R> = ClientStreamWriter<T, R>;
pub type ReaderWriter<TIn, TOut> = BidiClientStream<TIn, TOut>;

pub struct Reader<T> {
    inner: SseClientStream<T>,
}

impl<T> Reader<T> {
    pub fn new(inner: SseClientStream<T>) -> Self {
        Self { inner }
    }

    pub async fn read(&mut self) -> Option<Result<T>> {
        self.inner.next().await
    }
}

pub struct BidiServerStream<TIn, TOut> {
    inbound: mpsc::Receiver<Result<TIn>>,
    outbound: Option<mpsc::Sender<Result<TOut>>>,
}

impl<TIn, TOut> BidiServerStream<TIn, TOut> {
    pub async fn read(&mut self) -> Option<Result<TIn>> {
        self.inbound.recv().await
    }

    pub async fn write(&mut self, item: TOut) -> Result<()> {
        let tx = self
            .outbound
            .as_mut()
            .ok_or_else(|| Error::new(500, "bidi stream is closed"))?;
        tx.send(Ok(item))
            .await
            .map_err(|_| Error::new(500, "bidi stream is closed"))
    }

    pub fn close(&mut self) {
        let _ = self.outbound.take();
    }

    pub fn error_sender(&self) -> Option<mpsc::Sender<Result<TOut>>> {
        self.outbound.as_ref().cloned()
    }
}

impl<TIn, TOut> Drop for BidiServerStream<TIn, TOut> {
    fn drop(&mut self) {
        let _ = self.outbound.take();
    }
}

pub struct BidiClientStream<TIn, TOut> {
    writer: Option<mpsc::Sender<Result<TIn>>>,
    reader: mpsc::Receiver<Result<TOut>>,
    write_task: Option<JoinHandle<()>>,
    read_task: Option<JoinHandle<()>>,
}

impl<TIn, TOut> BidiClientStream<TIn, TOut> {
    pub async fn write(&mut self, item: TIn) -> Result<()> {
        let tx = self
            .writer
            .as_mut()
            .ok_or_else(|| Error::new(500, "bidi stream is closed"))?;
        tx.send(Ok(item))
            .await
            .map_err(|_| Error::new(500, "bidi stream is closed"))
    }

    pub async fn read(&mut self) -> Option<Result<TOut>> {
        self.reader.recv().await
    }

    pub fn close(&mut self) {
        let _ = self.writer.take();
    }

    pub fn cancel(&mut self) {
        let _ = self.writer.take();
        if let Some(handle) = self.write_task.take() {
            handle.abort();
        }
        if let Some(handle) = self.read_task.take() {
            handle.abort();
        }
    }
}

impl<TIn, TOut> Drop for BidiClientStream<TIn, TOut> {
    fn drop(&mut self) {
        let _ = self.writer.take();
    }
}

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

pub fn open_bidi_server<TIn, TOut>(socket: AxumWebSocket) -> BidiServerStream<TIn, TOut>
where
    TIn: DeserializeOwned + Send + 'static,
    TOut: Serialize + Send + 'static,
{
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (in_tx, in_rx) = mpsc::channel::<Result<TIn>>(32);
    let (out_tx, mut out_rx) = mpsc::channel::<Result<TOut>>(32);

    let _read_task = tokio::spawn(async move {
        while let Some(msg) = ws_rx.next().await {
            let msg = match msg {
                Ok(value) => value,
                Err(err) => {
                    let _ = in_tx.send(Err(Error::new(500, err.to_string()))).await;
                    break;
                }
            };
            match msg {
                AxumWsMessage::Text(text) => match serde_json::from_str::<TIn>(&text) {
                    Ok(value) => {
                        if in_tx.send(Ok(value)).await.is_err() {
                            break;
                        }
                    }
                    Err(err) => {
                        let _ = in_tx
                            .send(Err(Error::new(400, format!("invalid ws payload: {err}"))))
                            .await;
                        break;
                    }
                },
                AxumWsMessage::Close(_) => break,
                _ => {}
            }
        }
    });

    let _write_task = tokio::spawn(async move {
        while let Some(item) = out_rx.recv().await {
            let item = match item {
                Ok(value) => value,
                Err(err) => {
                    let _ = ws_tx
                        .send(AxumWsMessage::Text(
                            serde_json::to_string(&ErrorBody::from(err))
                                .unwrap_or_else(|_| r#"{"code":500,"msg":"stream error"}"#.into())
                                .into(),
                        ))
                        .await;
                    break;
                }
            };
            let text = match serde_json::to_string(&item) {
                Ok(value) => value,
                Err(err) => {
                    let _ = ws_tx
                        .send(AxumWsMessage::Text(
                            serde_json::to_string(&ErrorBody::from(Error::new(
                                500,
                                err.to_string(),
                            )))
                            .unwrap_or_else(|_| r#"{"code":500,"msg":"stream error"}"#.into())
                            .into(),
                        ))
                        .await;
                    break;
                }
            };
            if ws_tx.send(AxumWsMessage::Text(text.into())).await.is_err() {
                break;
            }
        }
        let _ = ws_tx.send(AxumWsMessage::Close(None)).await;
    });

    BidiServerStream {
        inbound: in_rx,
        outbound: Some(out_tx),
    }
}

pub async fn open_bidi_client<TIn, TOut>(ws_url: &str) -> Result<BidiClientStream<TIn, TOut>>
where
    TIn: Serialize + Send + 'static,
    TOut: DeserializeOwned + Send + 'static,
{
    let (socket, _) = tokio_tungstenite::connect_async(ws_url)
        .await
        .map_err(|err| Error::new(500, err.to_string()))?;
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (write_tx, mut write_rx) = mpsc::channel::<Result<TIn>>(32);
    let (read_tx, read_rx) = mpsc::channel::<Result<TOut>>(32);
    let read_tx_writer = read_tx.clone();

    let write_task = tokio::spawn(async move {
        while let Some(item) = write_rx.recv().await {
            let item = match item {
                Ok(value) => value,
                Err(err) => {
                    let _ = read_tx_writer.send(Err(err)).await;
                    break;
                }
            };
            let text = match serde_json::to_string(&item) {
                Ok(value) => value,
                Err(err) => {
                    let _ = read_tx_writer
                        .send(Err(Error::new(500, err.to_string())))
                        .await;
                    break;
                }
            };
            if ws_tx
                .send(TungsteniteMessage::Text(text.into()))
                .await
                .is_err()
            {
                break;
            }
        }
        let _ = ws_tx.send(TungsteniteMessage::Close(None)).await;
    });

    let read_task = tokio::spawn(async move {
        while let Some(msg) = ws_rx.next().await {
            let msg = match msg {
                Ok(value) => value,
                Err(err) => {
                    let _ = read_tx.send(Err(Error::new(500, err.to_string()))).await;
                    break;
                }
            };
            match msg {
                TungsteniteMessage::Text(text) => match serde_json::from_str::<TOut>(&text) {
                    Ok(value) => {
                        if read_tx.send(Ok(value)).await.is_err() {
                            break;
                        }
                    }
                    Err(err) => {
                        if let Ok(body) = serde_json::from_str::<ErrorBody>(&text) {
                            let _ = read_tx.send(Err(Error::new(body.code, body.msg))).await;
                            break;
                        }
                        let _ = read_tx
                            .send(Err(Error::new(400, format!("invalid ws payload: {err}"))))
                            .await;
                        break;
                    }
                },
                TungsteniteMessage::Close(_) => break,
                _ => {}
            }
        }
    });

    Ok(BidiClientStream {
        writer: Some(write_tx),
        reader: read_rx,
        write_task: Some(write_task),
        read_task: Some(read_task),
    })
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
