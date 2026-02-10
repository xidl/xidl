mod session;

use crate::{AsyncStream, Error};
use serde_json::Value;
use session::ServerSession;
use std::{net::SocketAddr, sync::Arc};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
#[cfg(feature = "tokio-net")]
use tokio::net::{TcpListener, ToSocketAddrs};
use tokio::sync::Mutex;

#[async_trait::async_trait]
pub trait Handler: Send + Sync {
    async fn handle(&self, method: &str, params: Value) -> Result<Value, Error>;
}

#[async_trait::async_trait]
impl<T> Handler for Arc<T>
where
    T: Handler,
{
    async fn handle(&self, method: &str, params: Value) -> Result<Value, Error> {
        (**self).handle(method, params).await
    }
}

pub struct Io<R, W> {
    pub reader: R,
    pub writer: W,
}

impl<R, W> Io<R, W> {
    pub fn new(reader: R, writer: W) -> Self {
        Self { reader, writer }
    }
}

pub struct MuxListener<R, W> {
    io: Mutex<Option<Io<R, W>>>,
}

impl<R, W> MuxListener<R, W> {
    pub fn from_io(io: Io<R, W>) -> Self {
        Self {
            io: Mutex::new(Some(io)),
        }
    }
}

struct MuxStream<R, W> {
    reader: R,
    writer: W,
}

impl<R, W> AsyncRead for MuxStream<R, W>
where
    R: AsyncRead + Unpin,
    W: Unpin,
{
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.get_mut();
        std::pin::Pin::new(&mut this.reader).poll_read(cx, buf)
    }
}

impl<R, W> AsyncWrite for MuxStream<R, W>
where
    R: Unpin,
    W: AsyncWrite + Unpin,
{
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let this = self.get_mut();
        std::pin::Pin::new(&mut this.writer).poll_write(cx, buf)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.get_mut();
        std::pin::Pin::new(&mut this.writer).poll_flush(cx)
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.get_mut();
        std::pin::Pin::new(&mut this.writer).poll_shutdown(cx)
    }

    fn poll_write_vectored(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        let this = self.get_mut();
        std::pin::Pin::new(&mut this.writer).poll_write_vectored(cx, bufs)
    }

    fn is_write_vectored(&self) -> bool {
        self.writer.is_write_vectored()
    }
}

#[async_trait::async_trait]
pub trait Listener: Send {
    async fn accept(
        &self,
    ) -> std::io::Result<(Box<dyn AsyncStream + Unpin + Send + 'static>, SocketAddr)>;
}

#[cfg(feature = "tokio-net")]
#[async_trait::async_trait]
impl Listener for TcpListener {
    async fn accept(
        &self,
    ) -> std::io::Result<(Box<dyn AsyncStream + Unpin + Send + 'static>, SocketAddr)> {
        let (stream, peer) = TcpListener::accept(self).await?;
        Ok((Box::new(stream), peer))
    }
}

#[async_trait::async_trait]
impl<R, W> Listener for MuxListener<R, W>
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
{
    async fn accept(
        &self,
    ) -> std::io::Result<(Box<dyn AsyncStream + Unpin + Send + 'static>, SocketAddr)> {
        let mut io = self.io.lock().await;
        let io = io.take().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "mux listener already consumed",
            )
        })?;

        Ok((
            Box::new(MuxStream {
                reader: io.reader,
                writer: io.writer,
            }),
            SocketAddr::from(([127, 0, 0, 1], 0)),
        ))
    }
}

struct MultiHandler {
    services: Vec<Box<dyn Handler>>,
}

#[async_trait::async_trait]
impl Handler for MultiHandler {
    async fn handle(&self, method: &str, params: Value) -> Result<Value, Error> {
        for service in &self.services {
            match service.handle(method, params.clone()).await {
                Ok(value) => return Ok(value),
                Err(err) => {
                    if err.is_method_not_found() {
                        continue;
                    }
                    return Err(err);
                }
            }
        }
        Err(Error::method_not_found(method))
    }
}

pub struct ServerBuilder {
    listener: Option<Box<dyn Listener>>,
    services: Vec<Box<dyn Handler>>,
}

pub struct Server {
    listener: Box<dyn Listener>,
    services: Vec<Box<dyn Handler>>,
}

impl Server {
    pub fn builder() -> ServerBuilder {
        ServerBuilder {
            listener: None,
            services: Vec::new(),
        }
    }

    pub async fn serve(self) -> Result<(), Error> {
        let handler = Arc::new(MultiHandler {
            services: self.services,
        });
        loop {
            let (stream, _peer) = match self.listener.accept().await {
                Ok(v) => v,
                Err(err) if err.kind() == std::io::ErrorKind::BrokenPipe => return Ok(()),
                Err(err) => return Err(err.into()),
            };

            let mut session = ServerSession::new(stream, handler.clone());
            session.run().await?;
        }
    }
}

impl ServerBuilder {
    pub fn with_listener<L>(mut self, listener: L) -> Self
    where
        L: Listener + 'static,
    {
        self.listener = Some(Box::new(listener));
        self
    }

    pub fn with_service<S>(mut self, service: S) -> Self
    where
        S: Handler + 'static,
    {
        self.services.push(Box::new(service));
        self
    }

    pub async fn serve(self) -> Result<(), Error> {
        let listener = self.listener.ok_or(Error::Protocol("missing listener"))?;
        let server = Server {
            listener,
            services: self.services,
        };
        server.serve().await
    }

    #[cfg(feature = "tokio-net")]
    pub async fn serve_on<A>(self, addr: A) -> Result<(), Error>
    where
        A: ToSocketAddrs,
    {
        if self.listener.is_some() {
            return Err(Error::Protocol("listener already set"));
        }
        let listener = TcpListener::bind(addr).await?;
        self.with_listener(listener).serve().await
    }
}
