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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn batch_with_multiple_valid_requests_returns_array_with_both_ids() {
    let (mut reader, mut writer) = open_raw_pair("batch-valid").await;
    write_json_line(
        &mut writer,
        &json!([
            { "jsonrpc": "2.0", "id": 1, "method": "echo", "params": { "a": 1 } },
            { "jsonrpc": "2.0", "id": 2, "method": "echo", "params": { "b": 2 } }
        ]),
    )
    .await
    .expect("write batch");

    let response = read_response(&mut reader).await;
    let items = response.as_array().expect("batch response is an array");
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["id"], json!(1));
    assert_eq!(items[0]["result"], json!({ "a": 1 }));
    assert_eq!(items[1]["id"], json!(2));
    assert_eq!(items[1]["result"], json!({ "b": 2 }));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn batch_with_mixed_items_reports_per_item_errors() {
    let (mut reader, mut writer) = open_raw_pair("batch-mixed").await;
    write_json_line(
        &mut writer,
        &json!([
            { "jsonrpc": "2.0", "id": 1, "method": "echo", "params": "ok" },
            { "jsonrpc": "2.0", "id": 2, "method": "nope", "params": null },
            { "jsonrpc": "1.0", "id": 3, "method": "echo", "params": "x" }
        ]),
    )
    .await
    .expect("write batch");

    let response = read_response(&mut reader).await;
    let items = response.as_array().expect("batch response is an array");
    assert_eq!(items.len(), 3);
    assert_eq!(items[0]["id"], json!(1));
    assert_eq!(items[0]["result"], json!("ok"));
    assert_eq!(items[1]["id"], json!(2));
    assert_eq!(items[1]["error"]["code"], json!(-32601));
    assert_eq!(items[2]["id"], json!(3));
    assert_eq!(items[2]["error"]["code"], json!(-32600));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn empty_batch_returns_invalid_request_error() {
    let (mut reader, mut writer) = open_raw_pair("batch-empty").await;
    write_json_line(&mut writer, &json!([]))
        .await
        .expect("write batch");

    let response = read_response(&mut reader).await;
    assert!(response.get("result").is_none());
    assert_eq!(response["id"], Value::Null);
    assert_eq!(response["error"]["code"], json!(-32600));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn batch_omits_responses_for_notifications() {
    let (mut reader, mut writer) = open_raw_pair("batch-notification").await;
    write_json_line(
        &mut writer,
        &json!([
            { "jsonrpc": "2.0", "method": "echo", "params": "ignored" },
            { "jsonrpc": "2.0", "id": 4, "method": "echo", "params": "kept" }
        ]),
    )
    .await
    .expect("write batch");

    let response = read_response(&mut reader).await;
    let items = response.as_array().expect("batch response is an array");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], json!(4));
    assert_eq!(items[0]["result"], json!("kept"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn notification_gets_no_response_and_connection_stays_usable() {
    let (mut reader, mut writer) = open_raw_pair("notification").await;
    write_json_line(
        &mut writer,
        &json!({ "jsonrpc": "2.0", "method": "echo", "params": "note" }),
    )
    .await
    .expect("write notification");
    write_json_line(
        &mut writer,
        &json!({ "jsonrpc": "2.0", "id": 9, "method": "echo", "params": "after" }),
    )
    .await
    .expect("write follow-up call");

    let response = read_response(&mut reader).await;
    assert_eq!(response["id"], json!(9));
    assert_eq!(response["result"], json!("after"));
    expect_silence(&mut reader).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn string_and_null_ids_are_echoed_verbatim() {
    let (mut reader, mut writer) = open_raw_pair("ids").await;

    write_json_line(
        &mut writer,
        &json!({ "jsonrpc": "2.0", "id": "abc", "method": "echo", "params": 1 }),
    )
    .await
    .expect("write string-id call");
    let response = read_response(&mut reader).await;
    assert_eq!(response["id"], json!("abc"));
    assert_eq!(response["result"], json!(1));

    write_json_line(
        &mut writer,
        &json!({ "jsonrpc": "2.0", "id": null, "method": "echo", "params": 2 }),
    )
    .await
    .expect("write null-id call");
    let response = read_response(&mut reader).await;
    assert_eq!(response["id"], Value::Null);
    assert_eq!(response["result"], json!(2));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn malformed_json_line_reports_parse_error_then_continues() {
    let (mut reader, mut writer) = open_raw_pair("malformed").await;
    writer
        .write_all(b"not-json\n")
        .await
        .expect("write malformed line");
    write_json_line(
        &mut writer,
        &json!({ "jsonrpc": "2.0", "id": 7, "method": "echo", "params": "ok" }),
    )
    .await
    .expect("write follow-up call");

    let first = read_response(&mut reader).await;
    assert_eq!(first["id"], Value::Null);
    assert_eq!(first["error"]["code"], json!(-32700));

    let second = read_response(&mut reader).await;
    assert_eq!(second["id"], json!(7));
    assert_eq!(second["result"], json!("ok"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn non_object_request_is_rejected_as_invalid_request() {
    let (mut reader, mut writer) = open_raw_pair("non-object").await;
    writer
        .write_all(b"42\n")
        .await
        .expect("write non-object request");

    let response = read_response(&mut reader).await;
    assert_eq!(response["id"], Value::Null);
    assert_eq!(response["error"]["code"], json!(-32600));
    assert!(
        response["error"]["message"]
            .as_str()
            .expect("error message is a string")
            .contains("object")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn missing_method_or_version_is_rejected_with_id_echoed() {
    let (mut reader, mut writer) = open_raw_pair("invalid-shapes").await;

    write_json_line(
        &mut writer,
        &json!({ "jsonrpc": "2.0", "id": 5, "params": null }),
    )
    .await
    .expect("write request without method");
    let response = read_response(&mut reader).await;
    assert_eq!(response["id"], json!(5));
    assert_eq!(response["error"]["code"], json!(-32600));
    assert!(
        response["error"]["message"]
            .as_str()
            .expect("error message is a string")
            .contains("method")
    );

    write_json_line(
        &mut writer,
        &json!({ "id": 6, "method": "echo", "params": null }),
    )
    .await
    .expect("write request without version");
    let response = read_response(&mut reader).await;
    assert_eq!(response["id"], json!(6));
    assert_eq!(response["error"]["code"], json!(-32600));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn large_batch_preserves_request_order() {
    let (mut reader, mut writer) = open_raw_pair("batch-large").await;
    let requests = (0..50)
        .map(|index| {
            json!({
                "jsonrpc": "2.0",
                "id": index,
                "method": "echo",
                "params": { "index": index },
            })
        })
        .collect::<Vec<_>>();
    write_json_line(&mut writer, &Value::Array(requests))
        .await
        .expect("write batch");

    let response = read_response(&mut reader).await;
    let items = response.as_array().expect("batch response is an array");
    assert_eq!(items.len(), 50);
    for (index, item) in items.iter().enumerate() {
        assert_eq!(item["id"], json!(index));
        assert_eq!(item["result"], json!({ "index": index }));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bidi_in_batch_is_rejected_per_item_and_connection_survives() {
    let uri = format!("inproc://{}", random_endpoint("batch-bidi"));
    let serve_uri = uri.clone();
    let server = tokio::spawn(async move {
        Server::builder()
            .with_service(EchoBidiHandler)
            .serve_on(&serve_uri)
            .await
    });
    let stream = connect_with_retry(&uri).await.expect("connect inproc");
    let (read_half, mut write_half) = tokio::io::split(stream);
    let mut reader = BufReader::new(read_half);

    write_json_line(
        &mut write_half,
        &json!([
            { "jsonrpc": "2.0", "id": 1, "method": "bidi", "params": null },
            { "jsonrpc": "2.0", "id": 2, "method": "echo", "params": "ok" }
        ]),
    )
    .await
    .expect("write batch");

    let response = read_response(&mut reader).await;
    let items = response.as_array().expect("batch response is an array");
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["id"], json!(1));
    assert_eq!(items[0]["error"]["code"], json!(-32600));
    assert!(
        items[0]["error"]["message"]
            .as_str()
            .expect("error message is a string")
            .contains("batch")
    );
    assert_eq!(items[1]["id"], json!(2));
    assert_eq!(items[1]["result"], json!("ok"));

    write_json_line(
        &mut write_half,
        &json!({ "jsonrpc": "2.0", "id": 3, "method": "echo", "params": "again" }),
    )
    .await
    .expect("write follow-up call");
    let response = read_response(&mut reader).await;
    assert_eq!(response["id"], json!(3));
    assert_eq!(response["result"], json!("again"));

    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bidi_with_id_streams_items_after_handshake() {
    let uri = format!("inproc://{}", random_endpoint("bidi-ack"));
    let serve_uri = uri.clone();
    let server = tokio::spawn(async move {
        Server::builder()
            .with_service(EchoBidiHandler)
            .serve_on(&serve_uri)
            .await
    });

    let stream = connect_with_retry(&uri).await.expect("connect inproc");
    let mut bidi = xidl_jsonrpc::stream::open_bidi_client(stream, "bidi")
        .await
        .expect("open bidi stream");

    // open_bidi_client already consumed the server's handshake
    // acknowledgement (result null, id 1), so the first value read is the
    // first echoed stream item.
    bidi.write(json!({ "n": 1 }))
        .await
        .expect("write stream item");
    let echoed = bidi
        .read()
        .await
        .expect("echo expected")
        .expect("echo read failed");
    assert_eq!(echoed, json!({ "n": 1 }));

    bidi.close().await.expect("close bidi stream");
    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn id_less_bidi_request_streams_without_ack() {
    let uri = format!("inproc://{}", random_endpoint("bidi-idless"));
    let serve_uri = uri.clone();
    let server = tokio::spawn(async move {
        Server::builder()
            .with_service(EchoBidiHandler)
            .serve_on(&serve_uri)
            .await
    });
    let stream = connect_with_retry(&uri).await.expect("connect inproc");
    let (read_half, mut write_half) = tokio::io::split(stream);
    let mut reader = BufReader::new(read_half);

    // Notification-style bidi request: no id, so the server must not emit an
    // ack and must switch straight to the stream.
    write_json_line(
        &mut write_half,
        &json!({ "jsonrpc": "2.0", "method": "bidi", "params": null }),
    )
    .await
    .expect("write id-less bidi request");
    write_json_line(&mut write_half, &json!({ "n": 2 }))
        .await
        .expect("write stream item");
    write_half.shutdown().await.expect("shutdown write half");

    let echoed = read_response(&mut reader).await;
    assert_eq!(echoed, json!({ "n": 2 }));

    // After the client shuts down its write half, the server's bidi handler
    // sees EOF and closes the connection: the client must observe EOF, not a
    // further frame.
    let mut line = String::new();
    let bytes = tokio::time::timeout(Duration::from_secs(10), reader.read_line(&mut line))
        .await
        .expect("timed out waiting for the server to close the stream")
        .expect("failed to read after shutdown");
    assert_eq!(bytes, 0, "expected stream close, got: {line:?}");
    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn oversized_json_frame_is_rejected_and_server_recovers() {
    let (mut reader, mut writer) = open_raw_pair("oversized").await;
    let oversized = format!("{}\n", "x".repeat(4 * 1024 * 1024 + 64));
    writer
        .write_all(oversized.as_bytes())
        .await
        .expect("write oversized line");
    write_json_line(
        &mut writer,
        &json!({ "jsonrpc": "2.0", "id": 8, "method": "echo", "params": "ok" }),
    )
    .await
    .expect("write follow-up call");

    let first = read_response(&mut reader).await;
    assert_eq!(first["id"], Value::Null);
    assert_eq!(first["error"]["code"], json!(-32700));
    let message = first["error"]["message"].as_str().expect("error message");
    assert!(
        message.contains("frame exceeds maximum length") && message.contains("4194304"),
        "oversized frame must report the frame limit, got: {message:?}"
    );

    let second = read_response(&mut reader).await;
    assert_eq!(second["id"], json!(8));
    assert_eq!(second["result"], json!("ok"));
}

/// Builds a valid JSON-RPC echo request whose wire line is exactly `len` bytes
/// (excluding the newline), padding `params` with `x` characters. Returns the
/// line and the number of padding characters it carries.
fn json_line_of_exact_len(len: usize) -> (String, usize) {
    let head = r#"{"jsonrpc":"2.0","id":1,"method":"echo","params":""#;
    let tail = r#""}"#;
    let content = len - head.len() - tail.len();
    assert!(content > 0, "target length {len} leaves no params padding");
    let mut line = String::with_capacity(len);
    line.push_str(head);
    line.push_str(&"x".repeat(content));
    line.push_str(tail);
    assert_eq!(line.len(), len);
    (line, content)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exact_max_json_frame_is_accepted() {
    let (mut reader, mut writer) = open_raw_pair("exact-max").await;
    let max = 4 * 1024 * 1024;
    let (mut exact, padding) = json_line_of_exact_len(max);
    exact.push('\n');
    writer
        .write_all(exact.as_bytes())
        .await
        .expect("write exact-max line");
    write_json_line(
        &mut writer,
        &json!({ "jsonrpc": "2.0", "id": 9, "method": "echo", "params": "ok" }),
    )
    .await
    .expect("write follow-up call");

    let first = read_response(&mut reader).await;
    assert_eq!(first["id"], json!(1));
    assert_eq!(
        first["result"].as_str().map(str::len),
        Some(padding),
        "params padding must round-trip in full"
    );

    let second = read_response(&mut reader).await;
    assert_eq!(second["id"], json!(9));
    assert_eq!(second["result"], json!("ok"));
}
