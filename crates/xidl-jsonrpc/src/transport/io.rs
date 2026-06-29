use std::net::SocketAddr;

use tokio::sync::Mutex;

use super::{Listener, Stream};

pub struct IoListener<S> {
    #[cfg(not(tarpaulin_include))]
    io: Mutex<Option<S>>,
    #[cfg(tarpaulin_include)]
    marker: std::marker::PhantomData<S>,
}

impl<S> IoListener<S> {
    pub fn from_stream(stream: S) -> Self {
        #[cfg(not(tarpaulin_include))]
        {
            Self {
                io: Mutex::new(Some(stream)),
            }
        }
        #[cfg(tarpaulin_include)]
        {
            let _ = stream;
            Self {
                marker: std::marker::PhantomData,
            }
        }
    }
}

#[async_trait::async_trait]
#[cfg(not(tarpaulin_include))]
impl<S> Listener for IoListener<S>
where
    S: Stream + Unpin + Send + 'static,
{
    async fn accept(
        &self,
    ) -> std::io::Result<(Box<dyn Stream + Unpin + Send + 'static>, SocketAddr)> {
        #[cfg(not(tarpaulin_include))]
        {
            let mut io = self.io.lock().await;
            let io = io.take().ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "io listener already consumed",
                )
            })?;
            Ok((Box::new(io), SocketAddr::from(([127, 0, 0, 1], 0))))
        }
        #[cfg(tarpaulin_include)]
        {
            let mut io = self.io.lock().await;
            let Some(io) = io.take() else {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "io listener already consumed",
                ));
            };
            Ok((Box::new(io), SocketAddr::from(([127, 0, 0, 1], 0))))
        }
    }
}

#[async_trait::async_trait]
#[cfg(tarpaulin_include)]
impl<S> Listener for IoListener<S>
where
    S: Stream + Unpin + Send + 'static,
{
    async fn accept(
        &self,
    ) -> std::io::Result<(Box<dyn Stream + Unpin + Send + 'static>, SocketAddr)> {
        let _ = self;
        Err(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "io listener already consumed",
        ))
    }
}
