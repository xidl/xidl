use crate::stream::ReaderWriter;
use crate::{Error, Handler};
use serde_json::{Value, json};

mod batch;
mod handlers;
mod run;

const MAX_FRAME_LEN: usize = 4 * 1024 * 1024;

struct SessionHandler;

#[async_trait::async_trait]
impl Handler for SessionHandler {
    async fn handle(&self, method: &str, params: Value) -> Result<Value, Error> {
        match method {
            "ok" => Ok(json!({ "echo": params })),
            "rpc" => Err(Error::invalid_params("bad params")),
            "io" => Err(Error::Io(std::io::Error::other("disk"))),
            _ => Err(Error::method_not_found(method)),
        }
    }
    fn accepts_bidi(&self, method: &str) -> bool {
        method == "bidi"
    }
    async fn handle_bidi(
        &self,
        _method: &str,
        params: Value,
        mut stream: ReaderWriter<Value, Value>,
    ) -> Result<(), Error> {
        stream.write(json!({ "stream": params })).await?;
        stream.close().await
    }
}

struct ErrorReader;

impl tokio::io::AsyncRead for ErrorReader {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        _buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Ready(Err(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "read failure",
        )))
    }
}

impl tokio::io::AsyncWrite for ErrorReader {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        _buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        std::task::Poll::Ready(Ok(0))
    }
    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        std::task::Poll::Ready(Ok(()))
    }
    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        std::task::Poll::Ready(Ok(()))
    }
}
