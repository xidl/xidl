#[cfg(tarpaulin_include)]
use std::marker::PhantomData;
use std::net::SocketAddr;

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::Mutex;

use crate::Io;

use super::{Listener, Stream};

pub struct IoListener<R, W> {
    #[cfg(not(tarpaulin_include))]
    io: Mutex<Option<Io<R, W>>>,
    #[cfg(tarpaulin_include)]
    marker: PhantomData<(R, W)>,
}

impl<R, W> IoListener<R, W> {
    pub fn from_io(io: Io<R, W>) -> Self {
        #[cfg(not(tarpaulin_include))]
        {
            Self {
                io: Mutex::new(Some(io)),
            }
        }
        #[cfg(tarpaulin_include)]
        {
            let _ = io;
            Self {
                marker: PhantomData,
            }
        }
    }
}

#[cfg(not(tarpaulin_include))]
struct IoStream<R, W> {
    reader: R,
    writer: W,
}

#[cfg(not(tarpaulin_include))]
impl<R, W> AsyncRead for IoStream<R, W>
where
    R: AsyncRead + Unpin,
    W: Unpin,
{
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.get_mut();
        std::pin::Pin::new(&mut this.reader).poll_read(cx, buf)
    }
}

#[cfg(not(tarpaulin_include))]
impl<R, W> AsyncWrite for IoStream<R, W>
where
    R: Unpin,
    W: AsyncWrite + Unpin,
{
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let this = self.get_mut();
        std::pin::Pin::new(&mut this.writer).poll_write(cx, buf)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.get_mut();
        std::pin::Pin::new(&mut this.writer).poll_flush(cx)
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.get_mut();
        std::pin::Pin::new(&mut this.writer).poll_shutdown(cx)
    }

    fn poll_write_vectored(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        let this = self.get_mut();
        std::pin::Pin::new(&mut this.writer).poll_write_vectored(cx, bufs)
    }

    fn is_write_vectored(&self) -> bool {
        self.writer.is_write_vectored()
    }
}

#[async_trait::async_trait]
#[cfg(not(tarpaulin_include))]
impl<R, W> Listener for IoListener<R, W>
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
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
            Ok((
                Box::new(IoStream {
                    reader: io.reader,
                    writer: io.writer,
                }),
                SocketAddr::from(([127, 0, 0, 1], 0)),
            ))
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
            Ok((
                Box::new(IoStream {
                    reader: io.reader,
                    writer: io.writer,
                }),
                SocketAddr::from(([127, 0, 0, 1], 0)),
            ))
        }
    }
}

#[async_trait::async_trait]
#[cfg(tarpaulin_include)]
impl<R, W> Listener for IoListener<R, W>
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
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
