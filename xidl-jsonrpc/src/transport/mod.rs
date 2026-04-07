mod inproc;
mod io;
#[cfg(all(feature = "transport-ipc", unix))]
mod ipc;
#[cfg(feature = "transport-quic")]
mod quic;
#[cfg(feature = "transport-tcp")]
mod tcp;
#[cfg(feature = "transport-tls")]
mod tls;
#[cfg(any(
    feature = "transport-tls",
    feature = "transport-websocket",
    feature = "transport-quic"
))]
mod tls_config;
#[cfg(feature = "transport-websocket")]
mod websocket;

use std::net::SocketAddr;

pub use inproc::{InprocListener, connect_inproc};
pub use io::IoListener;
#[cfg(all(feature = "transport-ipc", unix))]
pub use ipc::{IpcListener, connect_ipc};
#[cfg(feature = "transport-quic")]
pub use quic::{QuicListener, connect_quic};
#[cfg(feature = "transport-tcp")]
pub use tcp::TcpListener;
#[cfg(feature = "transport-tls")]
pub use tls::{TlsListener, connect_tls};
#[cfg(feature = "transport-websocket")]
pub use websocket::{WebSocketListener, connect_websocket};

pub trait Stream: tokio::io::AsyncRead + tokio::io::AsyncWrite {}

impl<T> Stream for T where T: tokio::io::AsyncRead + tokio::io::AsyncWrite {}

pub(crate) fn loopback_peer_addr() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], 0))
}

#[cfg(any(
    windows,
    not(unix),
    not(feature = "transport-tcp"),
    not(feature = "transport-quic"),
    not(feature = "transport-tls"),
    not(feature = "transport-websocket")
))]
pub(crate) fn unsupported(message: &'static str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Unsupported, message)
}

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

    async fn bind(self) -> std::io::Result<BoundListener> {
        let endpoint = self.display();
        let listener: Box<dyn Listener> = match self {
            Endpoint::Inproc(name) => Box::new(InprocListener::bind(name)?),
            Endpoint::Ipc(path) => {
                #[cfg(feature = "transport-ipc")]
                {
                    Box::new(IpcListener::bind(path)?)
                }
                #[cfg(not(feature = "transport-ipc"))]
                {
                    let _ = path;
                    return Err(unsupported("ipc transport is unsupported on windows"));
                }
            }
            Endpoint::Quic(endpoint) => {
                #[cfg(feature = "transport-quic")]
                {
                    Box::new(QuicListener::bind(&endpoint)?)
                }
                #[cfg(not(feature = "transport-quic"))]
                {
                    let _ = endpoint;
                    return Err(unsupported(
                        "quic transport requires `transport-quic` feature",
                    ));
                }
            }
            Endpoint::Tls(endpoint) => {
                #[cfg(feature = "transport-tls")]
                {
                    Box::new(TlsListener::bind(&endpoint).await?)
                }
                #[cfg(not(feature = "transport-tls"))]
                {
                    let _ = endpoint;
                    return Err(unsupported(
                        "tls transport requires `transport-tls` feature",
                    ));
                }
            }
            Endpoint::WebSocket(endpoint) => {
                #[cfg(feature = "transport-websocket")]
                {
                    Box::new(WebSocketListener::bind(&endpoint).await?)
                }
                #[cfg(not(feature = "transport-websocket"))]
                {
                    let _ = endpoint;
                    return Err(unsupported(
                        "websocket transport requires `transport-websocket` feature",
                    ));
                }
            }
            Endpoint::Tcp(addr) => {
                #[cfg(feature = "transport-tcp")]
                {
                    Box::new(TcpListener::bind(&addr).await?)
                }
                #[cfg(not(feature = "transport-tcp"))]
                {
                    let _ = addr;
                    return Err(unsupported(
                        "tcp transport requires `transport-tcp` feature",
                    ));
                }
            }
        };

        let resolved = listener.endpoint().unwrap_or(endpoint);
        Ok(BoundListener {
            listener,
            endpoint: resolved,
        })
    }

    async fn connect(self) -> std::io::Result<Box<dyn Stream + Unpin + Send + 'static>> {
        match self {
            Endpoint::Inproc(name) => Ok(Box::new(connect_inproc(&name)?)),
            Endpoint::Ipc(path) => {
                #[cfg(all(feature = "transport-ipc", unix))]
                {
                    connect_ipc(&path).await
                }
                #[cfg(all(feature = "transport-tcp", windows))]
                {
                    let _ = path;
                    Err(unsupported("ipc transport is unsupported on windows"))
                }
                #[cfg(not(feature = "transport-ipc"))]
                {
                    let _ = path;
                    Err(unsupported(
                        "ipc transport requires `transport-ipc` feature",
                    ))
                }
                #[cfg(all(feature = "transport-ipc", not(any(unix, windows))))]
                {
                    let _ = path;
                    Err(unsupported("ipc transport is unsupported on this platform"))
                }
            }
            Endpoint::Quic(endpoint) => {
                #[cfg(feature = "transport-quic")]
                {
                    connect_quic(&endpoint).await
                }
                #[cfg(not(feature = "transport-quic"))]
                {
                    let _ = endpoint;
                    Err(unsupported(
                        "quic transport requires `transport-quic` feature",
                    ))
                }
            }
            Endpoint::WebSocket(endpoint) => {
                #[cfg(feature = "transport-websocket")]
                {
                    connect_websocket(&endpoint).await
                }
                #[cfg(not(feature = "transport-websocket"))]
                {
                    let _ = endpoint;
                    Err(unsupported(
                        "websocket transport requires `transport-websocket` feature",
                    ))
                }
            }
            Endpoint::Tls(endpoint) => {
                #[cfg(feature = "transport-tls")]
                {
                    connect_tls(&endpoint).await
                }
                #[cfg(not(feature = "transport-tls"))]
                {
                    let _ = endpoint;
                    Err(unsupported(
                        "tls transport requires `transport-tls` feature",
                    ))
                }
            }
            Endpoint::Tcp(addr) => {
                #[cfg(feature = "transport-tcp")]
                {
                    let stream = tokio::net::TcpStream::connect(&addr).await?;
                    Ok(Box::new(stream))
                }
                #[cfg(not(feature = "transport-tcp"))]
                {
                    let _ = addr;
                    Err(unsupported(
                        "tcp transport requires `transport-tcp` feature",
                    ))
                }
            }
        }
    }

    fn display(&self) -> String {
        match self {
            Endpoint::Inproc(name) => format!("inproc://{name}"),
            Endpoint::Ipc(path) => format!("ipc://{path}"),
            Endpoint::Quic(endpoint) | Endpoint::Tls(endpoint) | Endpoint::WebSocket(endpoint) => {
                endpoint.clone()
            }
            Endpoint::Tcp(addr) => format!("tcp://{addr}"),
        }
    }
}

pub async fn bind(endpoint: &str) -> std::io::Result<BoundListener> {
    Endpoint::parse(endpoint).bind().await
}

pub async fn connect(endpoint: &str) -> std::io::Result<Box<dyn Stream + Unpin + Send + 'static>> {
    Endpoint::parse(endpoint).connect().await
}
