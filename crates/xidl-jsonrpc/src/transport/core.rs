use std::net::SocketAddr;

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
    pub(crate) fn new(listener: Box<dyn Listener>, endpoint: String) -> Self {
        Self { listener, endpoint }
    }

    pub fn into_parts(self) -> (Box<dyn Listener>, String) {
        (self.listener, self.endpoint)
    }
}
