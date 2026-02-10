use std::net::SocketAddr;

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::Mutex;
use xidl_jsonrpc::{AsyncStream, Io, Listener};

pub struct MuxListener<R, W> {
    io: Mutex<Option<Io<R, W>>>,
}

impl<R, W> MuxListener<R, W> {
    pub fn from_io(io: Io<R, W>) -> Self {
        Self {
            io: Mutex::new(Some(io)),
        }
    }
}

struct MuxStream<R, W> {
    reader: R,
    writer: W,
}

impl<R, W> AsyncRead for MuxStream<R, W>
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

impl<R, W> AsyncWrite for MuxStream<R, W>
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
impl<R, W> Listener for MuxListener<R, W>
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
{
    async fn accept(
        &self,
    ) -> std::io::Result<(Box<dyn AsyncStream + Unpin + Send + 'static>, SocketAddr)> {
        let mut io = self.io.lock().await;
        let io = io.take().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "mux listener already consumed",
            )
        })?;

        Ok((
            Box::new(MuxStream {
                reader: io.reader,
                writer: io.writer,
            }),
            SocketAddr::from(([127, 0, 0, 1], 0)),
        ))
    }
}
