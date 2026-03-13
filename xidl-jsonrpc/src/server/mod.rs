mod session;

use crate::Error;
#[cfg(all(feature = "tokio-net", unix))]
use crate::transport::IpcListener;
#[cfg(feature = "quic-s2n")]
use crate::transport::QuicListener;
#[cfg(feature = "tokio-net")]
use crate::transport::TcpListener;
#[cfg(feature = "tokio-tls")]
use crate::transport::TlsListener;
#[cfg(feature = "tokio-websocket")]
use crate::transport::WebSocketListener;
use crate::transport::{InprocListener, IoListener, Listener};
use serde_json::Value;
use session::ServerSession;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};

#[async_trait::async_trait]
pub trait Handler: Send + Sync {
    async fn handle(&self, method: &str, params: Value) -> Result<Value, Error>;

    fn accepts_bidi(&self, _method: &str) -> bool {
        false
    }

    async fn handle_bidi(
        &self,
        method: &str,
        _params: Value,
        _stream: crate::stream::ReaderWriter<Value, Value>,
    ) -> Result<(), Error> {
        Err(Error::method_not_found(method))
    }
}

#[async_trait::async_trait]
impl<T> Handler for Arc<T>
where
    T: Handler,
{
    async fn handle(&self, method: &str, params: Value) -> Result<Value, Error> {
        (**self).handle(method, params).await
    }

    fn accepts_bidi(&self, method: &str) -> bool {
        (**self).accepts_bidi(method)
    }

    async fn handle_bidi(
        &self,
        method: &str,
        params: Value,
        stream: crate::stream::ReaderWriter<Value, Value>,
    ) -> Result<(), Error> {
        (**self).handle_bidi(method, params, stream).await
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

    fn accepts_bidi(&self, method: &str) -> bool {
        self.services
            .iter()
            .any(|service| service.accepts_bidi(method))
    }

    async fn handle_bidi(
        &self,
        method: &str,
        params: Value,
        stream: crate::stream::ReaderWriter<Value, Value>,
    ) -> Result<(), Error> {
        for service in &self.services {
            if service.accepts_bidi(method) {
                return service.handle_bidi(method, params, stream).await;
            }
        }
        Err(Error::method_not_found(method))
    }
}

pub struct ServerBuilder {
    listener: Option<Box<dyn Listener>>,
    endpoint: Option<String>,
    services: Vec<Box<dyn Handler>>,
}

pub struct Server {
    listener: Box<dyn Listener>,
    endpoint: Option<String>,
    services: Vec<Box<dyn Handler>>,
}

impl Server {
    pub fn builder() -> ServerBuilder {
        ServerBuilder {
            listener: None,
            endpoint: None,
            services: Vec::new(),
        }
    }

    pub fn endpoint(&self) -> Option<&str> {
        self.endpoint.as_deref()
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
            let handler = handler.clone();
            tokio::spawn(async move {
                let mut session = ServerSession::new(stream, handler);
                let _ = session.run().await;
            });
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

    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    pub fn with_service<S>(mut self, service: S) -> Self
    where
        S: Handler + 'static,
    {
        self.services.push(Box::new(service));
        self
    }

    pub fn with_io<R, W>(self, io: Io<R, W>) -> Self
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        self.with_listener(IoListener::from_io(io))
    }

    pub async fn build(self) -> Result<Server, Error> {
        if self.listener.is_some() && self.endpoint.is_some() {
            return Err(Error::Protocol("listener already set"));
        }

        let (listener, endpoint) = if let Some(listener) = self.listener {
            (listener, None)
        } else if let Some(endpoint) = self.endpoint {
            Self::bind_listener(&endpoint).await?
        } else {
            return Err(Error::Protocol("missing listener"));
        };

        Ok(Server {
            listener,
            endpoint,
            services: self.services,
        })
    }

    pub async fn build_on<S>(self, endpoint: S) -> Result<Server, Error>
    where
        S: AsRef<str>,
    {
        self.with_endpoint(endpoint.as_ref()).build().await
    }

    pub async fn serve(self) -> Result<(), Error> {
        self.build().await?.serve().await
    }

    pub async fn serve_on<S>(self, endpoint: S) -> Result<(), Error>
    where
        S: AsRef<str>,
    {
        self.build_on(endpoint).await?.serve().await
    }

    async fn bind_listener(endpoint: &str) -> Result<(Box<dyn Listener>, Option<String>), Error> {
        let listener: Box<dyn Listener> = if let Some(addr) = endpoint.strip_prefix("tcp://") {
            #[cfg(feature = "tokio-net")]
            {
                Box::new(TcpListener::bind(addr).await?)
            }
            #[cfg(not(feature = "tokio-net"))]
            {
                let _ = addr;
                return Err(Error::Protocol(
                    "tcp transport requires `tokio-net` feature",
                ));
            }
        } else if let Some(path) = endpoint.strip_prefix("ipc://") {
            #[cfg(all(feature = "tokio-net", unix))]
            {
                Box::new(IpcListener::bind(path)?)
            }
            #[cfg(all(feature = "tokio-net", windows))]
            {
                let _ = path;
                return Err(Error::Protocol("ipc transport is unsupported on windows"));
            }
            #[cfg(not(feature = "tokio-net"))]
            {
                let _ = path;
                return Err(Error::Protocol(
                    "ipc transport requires `tokio-net` feature",
                ));
            }
            #[cfg(all(feature = "tokio-net", not(any(unix, windows))))]
            {
                let _ = path;
                return Err(Error::Protocol(
                    "ipc transport is unsupported on this platform",
                ));
            }
        } else if endpoint.starts_with("quic://") {
            #[cfg(feature = "quic-s2n")]
            {
                Box::new(QuicListener::bind(endpoint)?)
            }
            #[cfg(not(feature = "quic-s2n"))]
            {
                return Err(Error::Protocol(
                    "quic transport requires `quic-s2n` feature",
                ));
            }
        } else if endpoint.starts_with("tls://") {
            #[cfg(feature = "tokio-tls")]
            {
                Box::new(TlsListener::bind(endpoint).await?)
            }
            #[cfg(not(feature = "tokio-tls"))]
            {
                return Err(Error::Protocol(
                    "tls transport requires `tokio-tls` feature",
                ));
            }
        } else if endpoint.starts_with("ws://") || endpoint.starts_with("wss://") {
            #[cfg(feature = "tokio-websocket")]
            {
                Box::new(WebSocketListener::bind(endpoint).await?)
            }
            #[cfg(not(feature = "tokio-websocket"))]
            {
                return Err(Error::Protocol(
                    "websocket transport requires `tokio-websocket` feature",
                ));
            }
        } else if let Some(endpoint) = endpoint.strip_prefix("inproc://") {
            Box::new(InprocListener::bind(endpoint.to_string())?)
        } else {
            #[cfg(feature = "tokio-net")]
            {
                Box::new(TcpListener::bind(endpoint).await?)
            }
            #[cfg(not(feature = "tokio-net"))]
            {
                return Err(Error::Protocol(
                    "tcp transport requires `tokio-net` feature",
                ));
            }
        };
        let resolved = listener.endpoint().unwrap_or_else(|| endpoint.to_string());
        Ok((listener, Some(resolved)))
    }
}
