use std::io;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

mod call;
mod concurrent;

/// A transport whose writes succeed until `fail_writes` is set, then fail
/// with `BrokenPipe`. Reads always park, so the client's reader task never
/// observes EOF or an error and cannot race the write-failure path.
struct FailingWriteStream {
    written: Arc<Mutex<Vec<u8>>>,
    fail_writes: Arc<AtomicBool>,
}

impl AsyncRead for FailingWriteStream {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Poll::Pending
    }
}

impl AsyncWrite for FailingWriteStream {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        if self.fail_writes.load(Ordering::Acquire) {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "test stream write failed",
            )));
        }
        self.written.lock().unwrap().extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

/// Waits until `needle` appears in the bytes the mock stream has accepted.
async fn wait_for_written(written: &Arc<Mutex<Vec<u8>>>, needle: &[u8]) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        if written
            .lock()
            .unwrap()
            .windows(needle.len())
            .any(|window| window == needle)
        {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "request bytes never reached the stream"
        );
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
}
