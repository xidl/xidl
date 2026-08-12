//! Advanced end-to-end JSON-RPC protocol integration tests.
//!
//! These tests drive the public `Server` and `stream` APIs over the real
//! inproc transport while writing raw newline-delimited JSON on the wire, so
//! they can cover protocol features the typed `Client` cannot express: batch
//! requests, notifications, invalid requests, and non-numeric ids.

#![cfg(feature = "tokio")]

use serde_json::{Value, json};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use xidl_jsonrpc::{Error, Handler, Server};

mod batch;
mod oversized;
mod stream;

type BoxedStream = Box<dyn xidl_jsonrpc::transport::Stream + Unpin + Send + 'static>;

fn random_endpoint(prefix: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before epoch")
        .as_nanos();
    format!("{prefix}-{nanos}")
}

async fn connect_with_retry(endpoint: &str) -> std::io::Result<BoxedStream> {
    let mut last_err = None;
    for _ in 0..50 {
        match xidl_jsonrpc::connect(endpoint).await {
            Ok(stream) => return Ok(stream),
            Err(err) => {
                last_err = Some(err);
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
    }
    Err(last_err.unwrap_or_else(|| {
        std::io::Error::other(format!("failed to connect endpoint: {endpoint}"))
    }))
}

struct EchoHandler;

#[async_trait::async_trait]
impl Handler for EchoHandler {
    async fn handle(&self, method: &str, params: Value) -> Result<Value, Error> {
        if method == "echo" {
            Ok(params)
        } else {
            Err(Error::method_not_found(method))
        }
    }
}

struct EchoBidiHandler;

#[async_trait::async_trait]
impl Handler for EchoBidiHandler {
    async fn handle(&self, method: &str, params: Value) -> Result<Value, Error> {
        if method == "echo" {
            Ok(params)
        } else {
            Err(Error::method_not_found(method))
        }
    }

    fn accepts_bidi(&self, method: &str) -> bool {
        method == "bidi"
    }

    async fn handle_bidi(
        &self,
        method: &str,
        _params: Value,
        mut stream: xidl_jsonrpc::stream::ReaderWriter<Value, Value>,
    ) -> Result<(), Error> {
        if method != "bidi" {
            return Err(Error::method_not_found(method));
        }
        while let Some(item) = stream.read().await {
            let value = item?;
            stream.write(value).await?;
        }
        stream.close().await
    }
}

/// Writes one newline-terminated JSON value on the wire.
async fn write_json_line<W>(writer: &mut W, value: &Value) -> std::io::Result<()>
where
    W: AsyncWriteExt + Unpin,
{
    let mut line = serde_json::to_string(value).map_err(std::io::Error::other)?;
    line.push('\n');
    writer.write_all(line.as_bytes()).await?;
    writer.flush().await
}

/// Reads one newline-terminated JSON value from the wire.
async fn read_json_line<R>(reader: &mut R) -> std::io::Result<Option<Value>>
where
    R: AsyncBufReadExt + Unpin,
{
    let mut line = String::new();
    if reader.read_line(&mut line).await? == 0 {
        return Ok(None);
    }
    Ok(Some(
        serde_json::from_str(&line).map_err(std::io::Error::other)?,
    ))
}

/// Reads one response line, failing the test if the server stays silent.
async fn read_response<R>(reader: &mut R) -> Value
where
    R: AsyncBufReadExt + Unpin,
{
    tokio::time::timeout(Duration::from_secs(10), read_json_line(reader))
        .await
        .expect("timed out waiting for a response")
        .expect("failed to read response")
        .expect("connection closed before a response arrived")
}

/// Asserts the server sends nothing else on the connection.
async fn expect_silence<R>(reader: &mut R)
where
    R: AsyncBufReadExt + Unpin,
{
    let mut line = String::new();
    let outcome =
        tokio::time::timeout(Duration::from_millis(300), reader.read_line(&mut line)).await;
    assert!(
        outcome.is_err(),
        "expected no further response, but received: {line:?}"
    );
}

async fn open_raw_pair(
    prefix: &str,
) -> (
    BufReader<tokio::io::ReadHalf<BoxedStream>>,
    tokio::io::WriteHalf<BoxedStream>,
) {
    let uri = format!("inproc://{}", random_endpoint(prefix));
    let serve_uri = uri.clone();
    let server = tokio::spawn(async move {
        Server::builder()
            .with_service(EchoHandler)
            .serve_on(&serve_uri)
            .await
    });
    let stream = connect_with_retry(&uri).await.expect("connect inproc");
    let (read_half, write_half) = tokio::io::split(stream);
    let reader = BufReader::new(read_half);
    drop(server);
    (reader, write_half)
}
