use super::{ServerSession, response::ResponseCodec};
use crate::stream::ReaderWriter;

const MAX_FRAME_LEN: usize = 4 * 1024 * 1024;

use crate::{Error, Handler};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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

#[tokio::test]
async fn run_handles_success_and_error_responses() {
    let (mut client, server) = tokio::io::duplex(1024);
    let mut session = ServerSession::with_codec(server, SessionHandler, crate::codec::Codec::Json);
    let task = tokio::spawn(async move { session.run().await.unwrap() });
    client
        .write_all(br#"{"jsonrpc":"2.0","id":1,"method":"ok","params":{"a":1}}"#)
        .await
        .unwrap();
    client.write_all(b"\n").await.unwrap();
    client
        .write_all(br#"{"jsonrpc":"2.0","id":2,"method":"rpc","params":null}"#)
        .await
        .unwrap();
    client.write_all(b"\n").await.unwrap();
    client.shutdown().await.unwrap();
    let mut output = String::new();
    client.read_to_string(&mut output).await.unwrap();
    task.await.unwrap();
    let responses = output
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(responses[0]["id"], json!(1));
    assert_eq!(responses[0]["result"], json!({ "echo": { "a": 1 } }));
    assert_eq!(responses[1]["id"], json!(2));
    assert_eq!(responses[1]["error"]["code"], json!(-32602));
    assert_eq!(responses[1]["error"]["message"], json!("bad params"));
}

#[tokio::test]
async fn run_handles_parse_and_protocol_errors() {
    let (mut client, server) = tokio::io::duplex(512);
    let mut session = ServerSession::with_codec(server, SessionHandler, crate::codec::Codec::Json);
    let task = tokio::spawn(async move { session.run().await.unwrap() });
    client.write_all(b"not-json\n").await.unwrap();
    client
        .write_all(br#"{"id":3,"params":null}"#)
        .await
        .unwrap();
    client.write_all(b"\n").await.unwrap();
    client.shutdown().await.unwrap();
    let mut output = String::new();
    client.read_to_string(&mut output).await.unwrap();
    task.await.unwrap();
    let responses = output
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(responses[0]["error"]["code"], json!(-32700));
    assert_eq!(responses[1]["id"], json!(3));
    assert_eq!(responses[1]["error"]["code"], json!(-32600));
    assert_eq!(
        responses[1]["error"]["message"],
        json!("missing or invalid jsonrpc version")
    );
}

#[tokio::test]
async fn bidi_requests_take_over_the_stream() {
    let (mut client, server) = tokio::io::duplex(512);
    let mut session = ServerSession::with_codec(server, SessionHandler, crate::codec::Codec::Json);
    let task = tokio::spawn(async move { session.run().await.unwrap() });
    client
        .write_all(br#"{"jsonrpc":"2.0","id":1,"method":"bidi","params":{"n":7}}"#)
        .await
        .unwrap();
    client.write_all(b"\n").await.unwrap();
    client.shutdown().await.unwrap();
    let mut output = String::new();
    client.read_to_string(&mut output).await.unwrap();
    task.await.unwrap();
    assert_eq!(
        output,
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":null}\n{\"stream\":{\"n\":7}}\n"
    );
}

#[tokio::test]
async fn private_helpers_map_errors_and_missing_streams() {
    let (_client, server) = tokio::io::duplex(64);
    let mut session = ServerSession::with_codec(server, SessionHandler, crate::codec::Codec::Json);
    assert_eq!(
        ResponseCodec::success(Some(json!(9)), json!(1)).id,
        Some(json!(9))
    );
    assert_eq!(
        ResponseCodec::error(Some(json!(2)), Error::Protocol("bad"))
            .error
            .unwrap()
            .code,
        -32600
    );
    assert_eq!(
        ResponseCodec::rpc_error(Error::Io(std::io::Error::other("io"))).code,
        -32603
    );
    assert_eq!(
        ResponseCodec::rpc_error(Error::invalid_params("bad")).code,
        -32602
    );
    session.writer = None;
    assert!(matches!(
        session.write_result(Some(json!(1)), Value::Null).await,
        Err(Error::Protocol("missing stream"))
    ));
    let request = super::ParsedRequest {
        id: Some(json!(1)),
        method: "bidi".to_string(),
        params: Value::Null,
    };
    assert!(matches!(
        session.handle_request(request).await,
        Err(Error::Protocol("missing stream"))
    ));
    session.reader = None;
    session.run().await.unwrap();
}

#[tokio::test]
async fn run_handles_empty_batch_with_single_error() {
    let (mut client, server) = tokio::io::duplex(512);
    let mut session = ServerSession::with_codec(server, SessionHandler, crate::codec::Codec::Json);
    let task = tokio::spawn(async move { session.run().await.unwrap() });
    client.write_all(b"[]\n").await.unwrap();
    client.shutdown().await.unwrap();
    let mut output = String::new();
    client.read_to_string(&mut output).await.unwrap();
    task.await.unwrap();
    let value: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
    assert!(
        value.is_object(),
        "empty batch must yield a single error object"
    );
    assert_eq!(value["id"], json!(Value::Null));
    assert_eq!(value["error"]["code"], json!(-32600));
    assert_eq!(value["error"]["message"], json!("empty batch"));
}

#[tokio::test]
async fn run_handles_mixed_batch_and_silences_notifications() {
    let (mut client, server) = tokio::io::duplex(512);
    let mut session = ServerSession::with_codec(server, SessionHandler, crate::codec::Codec::Json);
    let task = tokio::spawn(async move { session.run().await.unwrap() });
    client
        .write_all(
            br#"[{"jsonrpc":"2.0","id":1,"method":"ok","params":{"a":1}},{"jsonrpc":"2.0","method":"notify","params":1}]"#,
        )
        .await
        .unwrap();
    client.write_all(b"\n").await.unwrap();
    client.shutdown().await.unwrap();
    let mut output = String::new();
    client.read_to_string(&mut output).await.unwrap();
    task.await.unwrap();
    let value: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
    assert!(value.is_array());
    let items = value.as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], json!(1));
    assert_eq!(items[0]["result"], json!({ "echo": { "a": 1 } }));
}

#[tokio::test]
async fn run_marks_invalid_batch_elements() {
    let (mut client, server) = tokio::io::duplex(512);
    let mut session = ServerSession::with_codec(server, SessionHandler, crate::codec::Codec::Json);
    let task = tokio::spawn(async move { session.run().await.unwrap() });
    client
        .write_all(br#"[{"jsonrpc":"2.0","id":1,"method":"ok","params":null},42,"bad"]"#)
        .await
        .unwrap();
    client.write_all(b"\n").await.unwrap();
    client.shutdown().await.unwrap();
    let mut output = String::new();
    client.read_to_string(&mut output).await.unwrap();
    task.await.unwrap();
    let value: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
    let items = value.as_array().unwrap();
    assert_eq!(items.len(), 3);
    assert_eq!(items[0]["id"], json!(1));
    assert_eq!(items[0]["result"], json!({ "echo": null }));
    assert_eq!(items[1]["error"]["code"], json!(-32600));
    assert_eq!(
        items[1]["error"]["message"],
        json!("request must be an object")
    );
    assert_eq!(items[2]["error"]["code"], json!(-32600));
    assert_eq!(
        items[2]["error"]["message"],
        json!("request must be an object")
    );
}

#[tokio::test]
async fn run_rejects_bidi_methods_in_batches() {
    let (mut client, server) = tokio::io::duplex(512);
    let mut session = ServerSession::with_codec(server, SessionHandler, crate::codec::Codec::Json);
    let task = tokio::spawn(async move { session.run().await.unwrap() });
    client
        .write_all(br#"[{"jsonrpc":"2.0","id":7,"method":"bidi","params":null}]"#)
        .await
        .unwrap();
    client.write_all(b"\n").await.unwrap();
    client.shutdown().await.unwrap();
    let mut output = String::new();
    client.read_to_string(&mut output).await.unwrap();
    task.await.unwrap();
    let value: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
    let items = value.as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], json!(7));
    assert_eq!(items[0]["error"]["code"], json!(-32600));
    assert_eq!(
        items[0]["error"]["message"],
        json!("bidi method not allowed in batch")
    );
}

#[tokio::test]
async fn run_continues_after_frame_too_large_errors() {
    let (mut client, server) = tokio::io::duplex(64 * 1024);
    let mut session = ServerSession::with_codec(server, SessionHandler, crate::codec::Codec::Json);
    let task = tokio::spawn(async move { session.run().await.unwrap() });
    let oversized = "x".repeat(MAX_FRAME_LEN + 1);
    client.write_all(oversized.as_bytes()).await.unwrap();
    client.write_all(b"\n").await.unwrap();
    client
        .write_all(br#"{"jsonrpc":"2.0","id":1,"method":"ok","params":null}"#)
        .await
        .unwrap();
    client.write_all(b"\n").await.unwrap();
    client.shutdown().await.unwrap();
    let mut output = String::new();
    client.read_to_string(&mut output).await.unwrap();
    task.await.unwrap();
    let responses = output
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(responses[0]["error"]["code"], json!(-32700));
    assert_eq!(
        responses[0]["error"]["message"],
        json!(format!(
            "frame exceeds maximum length {} bytes (json)",
            MAX_FRAME_LEN
        ))
    );
    assert_eq!(responses[1]["id"], json!(1));
    assert_eq!(responses[1]["result"], json!({ "echo": null }));
}

#[tokio::test]
async fn run_reports_missing_method() {
    let (mut client, server) = tokio::io::duplex(512);
    let mut session = ServerSession::with_codec(server, SessionHandler, crate::codec::Codec::Json);
    let task = tokio::spawn(async move { session.run().await.unwrap() });
    client
        .write_all(br#"{"jsonrpc":"2.0","id":5,"params":null}"#)
        .await
        .unwrap();
    client.write_all(b"\n").await.unwrap();
    client.shutdown().await.unwrap();
    let mut output = String::new();
    client.read_to_string(&mut output).await.unwrap();
    task.await.unwrap();
    let value: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
    assert_eq!(value["id"], json!(5));
    assert_eq!(value["error"]["code"], json!(-32600));
    assert_eq!(value["error"]["message"], json!("missing method"));
}

#[tokio::test]
async fn run_silences_bidi_notifications_in_batches() {
    let (mut client, server) = tokio::io::duplex(512);
    let mut session = ServerSession::with_codec(server, SessionHandler, crate::codec::Codec::Json);
    let task = tokio::spawn(async move { session.run().await.unwrap() });
    client
        .write_all(br#"[{"jsonrpc":"2.0","method":"bidi","params":null}]"#)
        .await
        .unwrap();
    client.write_all(b"\n").await.unwrap();
    client.shutdown().await.unwrap();
    let mut output = String::new();
    client.read_to_string(&mut output).await.unwrap();
    task.await.unwrap();
    assert!(
        output.is_empty(),
        "bidi notification in a batch must be silent"
    );
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

#[tokio::test]
async fn run_propagates_non_decode_read_errors() {
    let mut session =
        ServerSession::with_codec(ErrorReader, SessionHandler, crate::codec::Codec::Json);
    let err = session.run().await.unwrap_err();
    assert!(matches!(err, Error::Io(_)));
}

#[tokio::test]
async fn write_responses_reports_missing_stream() {
    let (_client, server) = tokio::io::duplex(64);
    let mut session = ServerSession::with_codec(server, SessionHandler, crate::codec::Codec::Json);
    session.writer = None;
    let items = vec![json!({"jsonrpc":"2.0","id":1,"method":"ok","params":null})];
    assert!(matches!(
        session.handle_batch(items).await,
        Err(Error::Protocol("missing stream"))
    ));
}

#[tokio::test]
async fn rpc_error_preserves_custom_rpc_payloads() {
    let custom = ResponseCodec::rpc_error(Error::Rpc {
        code: crate::ErrorCode::Custom(42),
        message: "custom failure".to_string(),
        data: Some(json!({"detail": 1})),
    });
    assert_eq!(custom.code, 42);
    assert_eq!(custom.message, "custom failure");
    assert_eq!(custom.data, Some(json!({"detail": 1})));
}
