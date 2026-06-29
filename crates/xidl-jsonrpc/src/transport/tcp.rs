use std::net::SocketAddr;

use super::{Listener, Stream};

pub struct TcpListener {
    inner: tokio::net::TcpListener,
}

impl TcpListener {
    pub async fn bind(addr: &str) -> std::io::Result<Self> {
        let inner = tokio::net::TcpListener::bind(addr).await?;
        Ok(Self { inner })
    }
}

#[async_trait::async_trait]
impl Listener for TcpListener {
    async fn accept(
        &self,
    ) -> std::io::Result<(Box<dyn Stream + Unpin + Send + 'static>, SocketAddr)> {
        let (stream, peer) = self.inner.accept().await?;
        Ok((Box::new(stream), peer))
    }

    fn endpoint(&self) -> Option<String> {
        self.inner
            .local_addr()
            .ok()
            .map(|addr| format!("tcp://{addr}"))
    }
}
