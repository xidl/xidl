mod inproc;
mod io;
#[cfg(all(feature = "tokio-net", unix))]
mod ipc;
#[cfg(feature = "quic-s2n")]
mod quic;
#[cfg(feature = "tokio-net")]
mod tcp;
#[cfg(feature = "tokio-tls")]
mod tls;
#[cfg(any(feature = "tokio-tls", feature = "tokio-websocket"))]
mod tls_config;
#[cfg(feature = "tokio-websocket")]
mod websocket;

use std::net::SocketAddr;

pub use inproc::{InprocListener, connect_inproc};
pub use io::IoListener;
#[cfg(all(feature = "tokio-net", unix))]
pub use ipc::{IpcListener, connect_ipc};
#[cfg(feature = "quic-s2n")]
pub use quic::{QuicListener, connect_quic};
#[cfg(feature = "tokio-net")]
pub use tcp::TcpListener;
#[cfg(feature = "tokio-tls")]
pub use tls::{TlsListener, connect_tls};
#[cfg(feature = "tokio-websocket")]
pub use websocket::{WebSocketListener, connect_websocket};

pub trait Stream: tokio::io::AsyncRead + tokio::io::AsyncWrite {}

impl<T> Stream for T where T: tokio::io::AsyncRead + tokio::io::AsyncWrite {}

#[async_trait::async_trait]
pub trait Listener: Send + Sync {
    async fn accept(
        &self,
    ) -> std::io::Result<(Box<dyn Stream + Unpin + Send + 'static>, SocketAddr)>;

    fn endpoint(&self) -> Option<String> {
        None
    }
}

pub struct BoundListener {
    listener: Box<dyn Listener>,
    endpoint: String,
}

impl BoundListener {
    pub fn into_parts(self) -> (Box<dyn Listener>, String) {
        (self.listener, self.endpoint)
    }
}

enum Endpoint {
    Inproc(String),
    Ipc(String),
    Quic(String),
    Tls(String),
    WebSocket(String),
    Tcp(String),
}

impl Endpoint {
    fn parse(endpoint: &str) -> Self {
        if let Some(name) = endpoint.strip_prefix("inproc://") {
            Self::Inproc(name.to_string())
        } else if let Some(path) = endpoint.strip_prefix("ipc://") {
            Self::Ipc(path.to_string())
        } else if endpoint.starts_with("quic://") {
            Self::Quic(endpoint.to_string())
        } else if endpoint.starts_with("tls://") {
            Self::Tls(endpoint.to_string())
        } else if endpoint.starts_with("ws://") || endpoint.starts_with("wss://") {
            Self::WebSocket(endpoint.to_string())
        } else if let Some(addr) = endpoint.strip_prefix("tcp://") {
            Self::Tcp(addr.to_string())
        } else {
            Self::Tcp(endpoint.to_string())
        }
    }
}

pub async fn bind(endpoint: &str) -> std::io::Result<BoundListener> {
    let listener: Box<dyn Listener> = match Endpoint::parse(endpoint) {
        Endpoint::Inproc(name) => Box::new(InprocListener::bind(name)?),
        Endpoint::Ipc(path) => {
            #[cfg(all(feature = "tokio-net", unix))]
            {
                Box::new(IpcListener::bind(path)?)
            }
            #[cfg(all(feature = "tokio-net", windows))]
            {
                let _ = path;
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "ipc transport is unsupported on windows",
                ));
            }
            #[cfg(not(feature = "tokio-net"))]
            {
                let _ = path;
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "ipc transport requires `tokio-net` feature",
                ));
            }
            #[cfg(all(feature = "tokio-net", not(any(unix, windows))))]
            {
                let _ = path;
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "ipc transport is unsupported on this platform",
                ));
            }
        }
        Endpoint::Quic(endpoint) => {
            #[cfg(feature = "quic-s2n")]
            {
                Box::new(QuicListener::bind(&endpoint)?)
            }
            #[cfg(not(feature = "quic-s2n"))]
            {
                let _ = endpoint;
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "quic transport requires `quic-s2n` feature",
                ));
            }
        }
        Endpoint::Tls(endpoint) => {
            #[cfg(feature = "tokio-tls")]
            {
                Box::new(TlsListener::bind(&endpoint).await?)
            }
            #[cfg(not(feature = "tokio-tls"))]
            {
                let _ = endpoint;
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "tls transport requires `tokio-tls` feature",
                ));
            }
        }
        Endpoint::WebSocket(endpoint) => {
            #[cfg(feature = "tokio-websocket")]
            {
                Box::new(WebSocketListener::bind(&endpoint).await?)
            }
            #[cfg(not(feature = "tokio-websocket"))]
            {
                let _ = endpoint;
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "websocket transport requires `tokio-websocket` feature",
                ));
            }
        }
        Endpoint::Tcp(addr) => {
            #[cfg(feature = "tokio-net")]
            {
                Box::new(TcpListener::bind(&addr).await?)
            }
            #[cfg(not(feature = "tokio-net"))]
            {
                let _ = addr;
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "tcp transport requires `tokio-net` feature",
                ));
            }
        }
    };

    let resolved = listener.endpoint().unwrap_or_else(|| endpoint.to_string());
    Ok(BoundListener {
        listener,
        endpoint: resolved,
    })
}

pub async fn connect(endpoint: &str) -> std::io::Result<Box<dyn Stream + Unpin + Send + 'static>> {
    match Endpoint::parse(endpoint) {
        Endpoint::Inproc(name) => Ok(Box::new(connect_inproc(&name)?)),
        Endpoint::Ipc(path) => {
            #[cfg(all(feature = "tokio-net", unix))]
            {
                connect_ipc(&path).await
            }
            #[cfg(all(feature = "tokio-net", windows))]
            {
                let _ = path;
                Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "ipc transport is unsupported on windows",
                ))
            }
            #[cfg(not(feature = "tokio-net"))]
            {
                let _ = path;
                Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "ipc transport requires `tokio-net` feature",
                ))
            }
            #[cfg(all(feature = "tokio-net", not(any(unix, windows))))]
            {
                let _ = path;
                Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "ipc transport is unsupported on this platform",
                ))
            }
        }
        Endpoint::Quic(endpoint) => {
            #[cfg(feature = "quic-s2n")]
            {
                connect_quic(&endpoint).await
            }
            #[cfg(not(feature = "quic-s2n"))]
            {
                let _ = endpoint;
                Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "quic transport requires `quic-s2n` feature",
                ))
            }
        }
        Endpoint::WebSocket(endpoint) => {
            #[cfg(feature = "tokio-websocket")]
            {
                connect_websocket(&endpoint).await
            }
            #[cfg(not(feature = "tokio-websocket"))]
            {
                let _ = endpoint;
                Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "websocket transport requires `tokio-websocket` feature",
                ))
            }
        }
        Endpoint::Tls(endpoint) => {
            #[cfg(feature = "tokio-tls")]
            {
                connect_tls(&endpoint).await
            }
            #[cfg(not(feature = "tokio-tls"))]
            {
                let _ = endpoint;
                Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "tls transport requires `tokio-tls` feature",
                ))
            }
        }
        Endpoint::Tcp(addr) => {
            #[cfg(feature = "tokio-net")]
            {
                let stream = tokio::net::TcpStream::connect(&addr).await?;
                Ok(Box::new(stream))
            }
            #[cfg(not(feature = "tokio-net"))]
            {
                let _ = addr;
                Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "tcp transport requires `tokio-net` feature",
                ))
            }
        }
    }
}
