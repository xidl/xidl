use std::net::SocketAddr;

use tokio::sync::Mutex;

use super::{Listener, Stream};

pub struct IoListener<S> {
    io: Mutex<Option<S>>,
}

impl<S> IoListener<S> {
    pub fn from_stream(stream: S) -> Self {
        Self {
            io: Mutex::new(Some(stream)),
        }
    }
}

#[async_trait::async_trait]
impl<S> Listener for IoListener<S>
where
    S: Stream + Unpin + Send + 'static,
{
    async fn accept(
        &self,
    ) -> std::io::Result<(Box<dyn Stream + Unpin + Send + 'static>, SocketAddr)> {
        let mut io = self.io.lock().await;
        let io = io.take().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "io listener already consumed",
            )
        })?;
        Ok((Box::new(io), SocketAddr::from(([127, 0, 0, 1], 0))))
    }
}
