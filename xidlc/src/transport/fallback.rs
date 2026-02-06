use std::{os::fd::FromRawFd, pin::pin};

use tokio::io::{AsyncRead, AsyncWrite};

pub fn pipe() -> Result<(Writer, Reader), std::io::Error> {
    let (tx, rx) = interprocess::unnamed_pipe::tokio::pipe()?;
    Ok((Writer(tx), Reader(rx)))
}

pub struct Reader(interprocess::unnamed_pipe::tokio::Recver);

impl Reader {
    pub fn into_owned_fd(self) -> std::os::fd::OwnedFd {
        let fd =
            unsafe { std::os::fd::OwnedFd::from_raw_fd(std::os::fd::AsRawFd::as_raw_fd(&self.0)) };

        std::mem::forget(self);

        fd
    }
}

impl AsyncRead for Reader {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        pin!(&mut self.0).poll_read(cx, buf)
    }
}

pub struct Writer(interprocess::unnamed_pipe::tokio::Sender);

impl Writer {
    pub fn into_owned_fd(self) -> std::os::fd::OwnedFd {
        let fd =
            unsafe { std::os::fd::OwnedFd::from_raw_fd(std::os::fd::AsRawFd::as_raw_fd(&self.0)) };

        std::mem::forget(self);

        fd
    }
}

impl AsyncWrite for Writer {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        pin!(&mut self.0).poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        pin!(&mut self.0).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        pin!(&mut self.0).poll_shutdown(cx)
    }
}
