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

pub async fn connect(endpoint: &str) -> std::io::Result<Box<dyn Stream + Unpin + Send + 'static>> {
    if let Some(name) = endpoint.strip_prefix("inproc://") {
        return Ok(Box::new(connect_inproc(name)?));
    }

    if let Some(path) = endpoint.strip_prefix("ipc://") {
        #[cfg(all(feature = "tokio-net", unix))]
        {
            return connect_ipc(path).await;
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

    #[cfg(feature = "quic-s2n")]
    if endpoint.starts_with("quic://") {
        return connect_quic(endpoint).await;
    }

    #[cfg(feature = "tokio-websocket")]
    if endpoint.starts_with("ws://") || endpoint.starts_with("wss://") {
        return connect_websocket(endpoint).await;
    }

    #[cfg(feature = "tokio-tls")]
    if endpoint.starts_with("tls://") {
        return connect_tls(endpoint).await;
    }

    #[cfg(feature = "tokio-net")]
    {
        let addr = endpoint.strip_prefix("tcp://").unwrap_or(endpoint);
        let stream = tokio::net::TcpStream::connect(addr).await?;
        #[allow(clippy::needless_return)]
        return Ok(Box::new(stream));
    }

    #[cfg(not(feature = "tokio-net"))]
    {
        let _ = endpoint;
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "tcp transport requires `tokio-net` feature",
        ))
    }
}
