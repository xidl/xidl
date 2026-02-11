use std::{os::windows::io::FromRawHandle, pin::pin};

use tokio::io::{AsyncRead, AsyncWrite};

pub fn pipe() -> Result<(Writer, Reader), std::io::Error> {
    let (tx, rx) = interprocess::unnamed_pipe::tokio::pipe()?;
    Ok((Writer(tx), Reader(rx)))
}

pub struct Reader(interprocess::unnamed_pipe::tokio::Recver);

impl Reader {
    pub fn into_owned_handle(self) -> std::os::windows::io::OwnedHandle {
        let handle = unsafe {
            std::os::windows::io::OwnedHandle::from_raw_handle(
                std::os::windows::io::AsRawHandle::as_raw_handle(&self.0),
            )
        };

        std::mem::forget(self);

        handle
    }

    pub fn into_stdio(self) -> std::process::Stdio {
        self.into_owned_handle().into()
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
    pub fn into_owned_handle(self) -> std::os::windows::io::OwnedHandle {
        let handle = unsafe {
            std::os::windows::io::OwnedHandle::from_raw_handle(
                std::os::windows::io::AsRawHandle::as_raw_handle(&self.0),
            )
        };

        std::mem::forget(self);

        handle
    }

    pub fn into_stdio(self) -> std::process::Stdio {
        self.into_owned_handle().into()
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
