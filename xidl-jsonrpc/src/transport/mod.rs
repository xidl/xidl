mod inproc;
mod io;
#[cfg(feature = "tokio-net")]
mod tcp;

use std::net::SocketAddr;

pub use inproc::{InprocListener, connect_inproc};
pub use io::IoListener;
#[cfg(feature = "tokio-net")]
pub use tcp::TcpListener;

pub trait Stream: tokio::io::AsyncRead + tokio::io::AsyncWrite {}

impl<T> Stream for T where T: tokio::io::AsyncRead + tokio::io::AsyncWrite {}

#[async_trait::async_trait]
pub trait Listener: Send + Sync {
    async fn accept(
        &self,
    ) -> std::io::Result<(Box<dyn Stream + Unpin + Send + 'static>, SocketAddr)>;
}
