mod core;
mod endpoint;
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

pub(crate) use core::loopback_peer_addr;
#[cfg(any(
    windows,
    not(unix),
    not(feature = "transport-tcp"),
    not(feature = "transport-quic"),
    not(feature = "transport-tls"),
    not(feature = "transport-websocket")
))]
pub(crate) use core::unsupported;
pub use core::{BoundListener, Listener, Stream};
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

pub async fn bind(endpoint: &str) -> std::io::Result<BoundListener> {
    endpoint::bind(endpoint).await
}

pub async fn connect(endpoint: &str) -> std::io::Result<Box<dyn Stream + Unpin + Send + 'static>> {
    endpoint::connect(endpoint).await
}
