use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    task::{self, Poll},
};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf, SimplexStream};

pub fn pipe() -> Result<(Writer, Reader), std::io::Error> {
    let one = Arc::new(Mutex::new(SimplexStream::new_unsplit(8192)));

    Ok((Writer { write: one.clone() }, Reader { read: one }))
}

pub struct Writer {
    write: Arc<Mutex<SimplexStream>>,
}

impl AsyncWrite for Writer {
    #[allow(unused_mut)]
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut *lock(&self.write)).poll_write(cx, buf)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> Poll<Result<usize, std::io::Error>> {
        Pin::new(&mut *lock(&self.write)).poll_write_vectored(cx, bufs)
    }

    fn is_write_vectored(&self) -> bool {
        true
    }

    #[allow(unused_mut)]
    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut *lock(&self.write)).poll_flush(cx)
    }

    #[allow(unused_mut)]
    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut *lock(&self.write)).poll_shutdown(cx)
    }
}

pub struct Reader {
    read: Arc<Mutex<SimplexStream>>,
}

impl AsyncRead for Reader {
    // Previous rustc required this `self` to be `mut`, even though newer
    // versions recognize it isn't needed to call `lock()`. So for
    // compatibility, we include the `mut` and `allow` the lint.
    //
    // See https://github.com/rust-lang/rust/issues/73592
    #[allow(unused_mut)]
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut *lock(&self.read)).poll_read(cx, buf)
    }
}

fn lock<T>(mtx: &Arc<Mutex<T>>) -> std::sync::MutexGuard<'_, T> {
    match mtx.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}
