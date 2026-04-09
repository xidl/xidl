use super::core::{BoundListener, Listener, Stream};
use super::{InprocListener, connect_inproc};

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
                #[cfg(all(feature = "transport-ipc", unix))]
                {
                    Box::new(super::IpcListener::bind(path)?)
                }
                #[cfg(all(feature = "transport-ipc", windows))]
                {
                    let _ = path;
                    return Err(super::unsupported(
                        "ipc transport is unsupported on windows",
                    ));
                }
                #[cfg(not(feature = "transport-ipc"))]
                {
                    let _ = path;
                    return Err(super::unsupported(
                        "ipc transport requires `transport-ipc` feature",
                    ));
                }
                #[cfg(all(feature = "transport-ipc", not(any(unix, windows))))]
                {
                    let _ = path;
                    return Err(super::unsupported(
                        "ipc transport is unsupported on this platform",
                    ));
                }
            }
            Endpoint::Quic(endpoint) => {
                #[cfg(feature = "transport-quic")]
                {
                    Box::new(super::QuicListener::bind(&endpoint)?)
                }
                #[cfg(not(feature = "transport-quic"))]
                {
                    let _ = endpoint;
                    return Err(super::unsupported(
                        "quic transport requires `transport-quic` feature",
                    ));
                }
            }
            Endpoint::Tls(endpoint) => {
                #[cfg(feature = "transport-tls")]
                {
                    Box::new(super::TlsListener::bind(&endpoint).await?)
                }
                #[cfg(not(feature = "transport-tls"))]
                {
                    let _ = endpoint;
                    return Err(super::unsupported(
                        "tls transport requires `transport-tls` feature",
                    ));
                }
            }
            Endpoint::WebSocket(endpoint) => {
                #[cfg(feature = "transport-websocket")]
                {
                    Box::new(super::WebSocketListener::bind(&endpoint).await?)
                }
                #[cfg(not(feature = "transport-websocket"))]
                {
                    let _ = endpoint;
                    return Err(super::unsupported(
                        "websocket transport requires `transport-websocket` feature",
                    ));
                }
            }
            Endpoint::Tcp(addr) => {
                #[cfg(feature = "transport-tcp")]
                {
                    Box::new(super::TcpListener::bind(&addr).await?)
                }
                #[cfg(not(feature = "transport-tcp"))]
                {
                    let _ = addr;
                    return Err(super::unsupported(
                        "tcp transport requires `transport-tcp` feature",
                    ));
                }
            }
        };

        let resolved = listener.endpoint().unwrap_or(endpoint);
        Ok(BoundListener::new(listener, resolved))
    }

    async fn connect(self) -> std::io::Result<Box<dyn Stream + Unpin + Send + 'static>> {
        match self {
            Endpoint::Inproc(name) => Ok(Box::new(connect_inproc(&name)?)),
            Endpoint::Ipc(path) => {
                #[cfg(all(feature = "transport-ipc", unix))]
                {
                    super::connect_ipc(&path).await
                }
                #[cfg(all(feature = "transport-tcp", windows))]
                {
                    let _ = path;
                    Err(super::unsupported(
                        "ipc transport is unsupported on windows",
                    ))
                }
                #[cfg(not(feature = "transport-ipc"))]
                {
                    let _ = path;
                    Err(super::unsupported(
                        "ipc transport requires `transport-ipc` feature",
                    ))
                }
                #[cfg(all(feature = "transport-ipc", not(any(unix, windows))))]
                {
                    let _ = path;
                    Err(super::unsupported(
                        "ipc transport is unsupported on this platform",
                    ))
                }
            }
            Endpoint::Quic(endpoint) => {
                #[cfg(feature = "transport-quic")]
                {
                    super::connect_quic(&endpoint).await
                }
                #[cfg(not(feature = "transport-quic"))]
                {
                    let _ = endpoint;
                    Err(super::unsupported(
                        "quic transport requires `transport-quic` feature",
                    ))
                }
            }
            Endpoint::WebSocket(endpoint) => {
                #[cfg(feature = "transport-websocket")]
                {
                    super::connect_websocket(&endpoint).await
                }
                #[cfg(not(feature = "transport-websocket"))]
                {
                    let _ = endpoint;
                    Err(super::unsupported(
                        "websocket transport requires `transport-websocket` feature",
                    ))
                }
            }
            Endpoint::Tls(endpoint) => {
                #[cfg(feature = "transport-tls")]
                {
                    super::connect_tls(&endpoint).await
                }
                #[cfg(not(feature = "transport-tls"))]
                {
                    let _ = endpoint;
                    Err(super::unsupported(
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
                    Err(super::unsupported(
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

pub(super) async fn bind(endpoint: &str) -> std::io::Result<BoundListener> {
    Endpoint::parse(endpoint).bind().await
}

pub(super) async fn connect(
    endpoint: &str,
) -> std::io::Result<Box<dyn Stream + Unpin + Send + 'static>> {
    Endpoint::parse(endpoint).connect().await
}
