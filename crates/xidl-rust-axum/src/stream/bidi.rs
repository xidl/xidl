use crate::{Error, ErrorBody, Result};
#[cfg(not(tarpaulin_include))]
use axum::extract::ws::{Message as AxumWsMessage, WebSocket as AxumWebSocket};
#[cfg(not(tarpaulin_include))]
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
#[cfg(not(tarpaulin_include))]
use serde::de::DeserializeOwned;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
#[cfg(not(tarpaulin_include))]
use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;
#[cfg(not(tarpaulin_include))]
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

/// Server-side handle for a bidirectional WebSocket stream.
pub struct BidiServerStream<TIn, TOut> {
    pub(super) inbound: mpsc::Receiver<Result<TIn>>,
    pub(super) outbound: Option<mpsc::Sender<Result<TOut>>>,
}

impl<TIn, TOut> BidiServerStream<TIn, TOut> {
    /// Reads the next client message.
    pub async fn read(&mut self) -> Option<Result<TIn>> {
        self.inbound.recv().await
    }

    /// Sends a message to the client.
    pub async fn write(&mut self, item: TOut) -> Result<()> {
        let tx = self
            .outbound
            .as_mut()
            .ok_or_else(|| Error::new(500, "bidi stream is closed"))?;
        tx.send(Ok(item))
            .await
            .map_err(|_| Error::new(500, "bidi stream is closed"))
    }

    /// Closes the outbound side of the stream.
    pub fn close(&mut self) {
        let _ = self.outbound.take();
    }

    /// Returns a sender that can emit terminal stream errors.
    pub fn error_sender(&self) -> Option<mpsc::Sender<Result<TOut>>> {
        self.outbound.as_ref().cloned()
    }
}

impl<TIn, TOut> Drop for BidiServerStream<TIn, TOut> {
    fn drop(&mut self) {
        let _ = self.outbound.take();
    }
}

/// Client-side handle for a bidirectional WebSocket stream.
pub struct BidiClientStream<TIn, TOut> {
    pub(super) writer: Option<mpsc::Sender<Result<TIn>>>,
    pub(super) reader: mpsc::Receiver<Result<TOut>>,
    pub(super) write_task: Option<JoinHandle<()>>,
    pub(super) read_task: Option<JoinHandle<()>>,
}

impl<TIn, TOut> BidiClientStream<TIn, TOut> {
    /// Sends a message to the server.
    pub async fn write(&mut self, item: TIn) -> Result<()> {
        let tx = self
            .writer
            .as_mut()
            .ok_or_else(|| Error::new(500, "bidi stream is closed"))?;
        tx.send(Ok(item))
            .await
            .map_err(|_| Error::new(500, "bidi stream is closed"))
    }

    /// Reads the next message from the server.
    pub async fn read(&mut self) -> Option<Result<TOut>> {
        self.reader.recv().await
    }

    /// Closes the outbound side of the stream.
    pub fn close(&mut self) {
        let _ = self.writer.take();
    }

    /// Aborts the background read and write tasks immediately.
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

/// Opens a server-side bidirectional WebSocket stream from an upgraded socket.
#[cfg(not(tarpaulin_include))]
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

/// Opens a client-side bidirectional WebSocket stream.
#[cfg(not(tarpaulin_include))]
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

/// Opens a client-side bidirectional WebSocket stream with custom headers.
#[cfg(not(tarpaulin_include))]
pub async fn open_bidi_client_with_headers<TIn, TOut>(
    ws_url: &str,
    headers: axum::http::HeaderMap,
) -> Result<BidiClientStream<TIn, TOut>>
where
    TIn: Serialize + Send + 'static,
    TOut: DeserializeOwned + Send + 'static,
{
    let mut req = ws_url
        .into_client_request()
        .map_err(|err| Error::new(500, err.to_string()))?;
    req.headers_mut().extend(headers);
    let (socket, _) = tokio_tungstenite::connect_async(req)
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
                        let _ = read_tx.send(Err(Error::new(500, err.to_string()))).await;
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
