use futures_core::Stream as _;
use futures_util::Sink;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::{Mutex, mpsc};
use tokio_tungstenite::tungstenite::Message;

use super::tls_config::{TransportUrl, build_client_config, build_server_acceptor};
use super::{Listener, Stream};
use crate::FrameKind;

type DynStream = Box<dyn Stream + Unpin + Send + 'static>;

enum ServerTls {
    Disabled,
    Enabled(tokio_rustls::TlsAcceptor),
}

pub struct WebSocketListener {
    rx: Mutex<mpsc::UnboundedReceiver<(DynStream, SocketAddr)>>,
    frame_kind: Arc<AtomicU8>,
    _accept_task: tokio::task::JoinHandle<()>,
}

impl WebSocketListener {
    pub async fn bind(endpoint: &str) -> std::io::Result<Self> {
        let endpoint = TransportUrl::parse(endpoint, &["ws", "wss"])?;
        let tls = if endpoint.scheme() == "wss" {
            let cert = endpoint.required_param("cert", "XIDL_WSS_CERT")?;
            let key = endpoint.required_param("key", "XIDL_WSS_KEY")?;
            ServerTls::Enabled(build_server_acceptor(&cert, &key)?)
        } else {
            ServerTls::Disabled
        };
        let listener = tokio::net::TcpListener::bind(endpoint.bind_addr()?).await?;
        let (tx, rx) = mpsc::unbounded_channel::<(DynStream, SocketAddr)>();
        let frame_kind = Arc::new(AtomicU8::new(0));
        let accepted_frame_kind = frame_kind.clone();
        let task = tokio::spawn(async move {
            loop {
                let (tcp, peer) = match listener.accept().await {
                    Ok(v) => v,
                    Err(_) => break,
                };
                let tx = tx.clone();
                let frame_kind = accepted_frame_kind.clone();
                let tls = match &tls {
                    ServerTls::Disabled => ServerTls::Disabled,
                    ServerTls::Enabled(acceptor) => ServerTls::Enabled(acceptor.clone()),
                };
                tokio::spawn(async move {
                    match tls {
                        ServerTls::Disabled => {
                            let ws = match tokio_tungstenite::accept_async(tcp).await {
                                Ok(ws) => ws,
                                Err(_) => return,
                            };
                            let kind = frame_kind.load(Ordering::Relaxed);
                            let stream = WebSocketIo::with_frame_kind(ws, decode_frame_kind(kind));
                            let _ = tx.send((Box::new(stream), peer));
                        }
                        ServerTls::Enabled(acceptor) => {
                            let tls = match acceptor.accept(tcp).await {
                                Ok(v) => v,
                                Err(_) => return,
                            };
                            let ws = match tokio_tungstenite::accept_async(tls).await {
                                Ok(ws) => ws,
                                Err(_) => return,
                            };
                            let kind = frame_kind.load(Ordering::Relaxed);
                            let stream = WebSocketIo::with_frame_kind(ws, decode_frame_kind(kind));
                            let _ = tx.send((Box::new(stream), peer));
                        }
                    }
                });
            }
        });
        Ok(Self {
            rx: Mutex::new(rx),
            frame_kind,
            _accept_task: task,
        })
    }
}

#[async_trait::async_trait]
impl Listener for WebSocketListener {
    async fn accept(&self) -> std::io::Result<(DynStream, SocketAddr)> {
        let mut rx = self.rx.lock().await;
        rx.recv().await.ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::BrokenPipe, "websocket listener closed")
        })
    }

    fn set_frame_kind(&self, kind: FrameKind) {
        self.frame_kind
            .store(encode_frame_kind(kind), Ordering::Relaxed);
    }
}

pub async fn connect_websocket(endpoint: &str) -> std::io::Result<DynStream> {
    let endpoint = TransportUrl::parse(endpoint, &["ws", "wss"])?;
    let connector = if endpoint.scheme() == "wss" {
        let ca = endpoint.required_param("ca", "XIDL_WSS_CA")?;
        let _ = endpoint.host_port()?;
        let config = build_client_config(&ca)?;
        Some(tokio_tungstenite::Connector::Rustls(config))
    } else {
        None
    };
    let (ws, _) =
        tokio_tungstenite::connect_async_tls_with_config(endpoint.as_str(), None, false, connector)
            .await
            .map_err(super::tls_config::io_other)?;
    Ok(Box::new(WebSocketIo::new(ws)))
}

pub struct WebSocketIo<S> {
    ws: tokio_tungstenite::WebSocketStream<S>,
    frame_kind: FrameKind,
    read_buf: Vec<u8>,
    write_buf: Vec<u8>,
}

impl<S> WebSocketIo<S> {
    /// Wraps a WebSocket using JSON text framing.
    pub fn new(ws: tokio_tungstenite::WebSocketStream<S>) -> Self {
        Self::with_frame_kind(ws, FrameKind::Text)
    }

    /// Wraps a WebSocket using the selected payload framing.
    pub fn with_frame_kind(
        ws: tokio_tungstenite::WebSocketStream<S>,
        frame_kind: FrameKind,
    ) -> Self {
        Self {
            ws,
            frame_kind,
            read_buf: Vec::new(),
            write_buf: Vec::new(),
        }
    }
}

impl<S> AsyncRead for WebSocketIo<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        loop {
            if !self.read_buf.is_empty() {
                let n = self.read_buf.len().min(buf.remaining());
                buf.put_slice(&self.read_buf[..n]);
                self.read_buf.drain(..n);
                return Poll::Ready(Ok(()));
            }

            match Pin::new(&mut self.ws).poll_next(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Some(Ok(Message::Text(text)))) => {
                    self.read_buf.extend_from_slice(text.as_bytes());
                    if !self.read_buf.ends_with(b"\n") {
                        self.read_buf.push(b'\n');
                    }
                }
                Poll::Ready(Some(Ok(Message::Binary(data)))) => {
                    self.read_buf.extend_from_slice(data.as_ref());
                }
                Poll::Ready(Some(Ok(Message::Ping(_))))
                | Poll::Ready(Some(Ok(Message::Pong(_))))
                | Poll::Ready(Some(Ok(Message::Frame(_)))) => {}
                Poll::Ready(Some(Ok(Message::Close(_)))) => return Poll::Ready(Ok(())),
                Poll::Ready(Some(Err(err))) => {
                    return Poll::Ready(Err(super::tls_config::io_other(err)));
                }
                Poll::Ready(None) => return Poll::Ready(Ok(())),
            }
        }
    }
}

impl<S> AsyncWrite for WebSocketIo<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        data: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.write_buf.extend_from_slice(data);
        Poll::Ready(Ok(data.len()))
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        if !self.write_buf.is_empty() {
            let payload = std::mem::take(&mut self.write_buf);
            match Pin::new(&mut self.ws).poll_ready(cx) {
                Poll::Pending => {
                    self.write_buf = payload;
                    return Poll::Pending;
                }
                Poll::Ready(Ok(())) => {}
                Poll::Ready(Err(err)) => {
                    self.write_buf = payload;
                    return Poll::Ready(Err(super::tls_config::io_other(err)));
                }
            }
            let message = match self.frame_kind {
                FrameKind::Text => match String::from_utf8(payload) {
                    Ok(text) => Message::Text(text.into()),
                    Err(err) => {
                        return Poll::Ready(Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            err.to_string(),
                        )));
                    }
                },
                FrameKind::Binary => Message::Binary(payload.into()),
            };
            if let Err(err) = Pin::new(&mut self.ws).start_send(message) {
                return Poll::Ready(Err(super::tls_config::io_other(err)));
            }
        }
        match Pin::new(&mut self.ws).poll_flush(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
            Poll::Ready(Err(err)) => Poll::Ready(Err(super::tls_config::io_other(err))),
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        if !self.write_buf.is_empty() {
            match self.as_mut().poll_flush(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
                Poll::Ready(Ok(())) => {}
            }
        }
        match Pin::new(&mut self.ws).poll_close(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
            Poll::Ready(Err(err)) => Poll::Ready(Err(super::tls_config::io_other(err))),
        }
    }
}

fn encode_frame_kind(kind: FrameKind) -> u8 {
    match kind {
        FrameKind::Text => 0,
        FrameKind::Binary => 1,
    }
}

fn decode_frame_kind(value: u8) -> FrameKind {
    if value == 1 {
        FrameKind::Binary
    } else {
        FrameKind::Text
    }
}
