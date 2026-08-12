use super::ServerSession;
use crate::stream::ReaderWriter;
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
    type TestSession = ServerSession<tokio::io::DuplexStream, SessionHandler>;

    assert_eq!(
        TestSession::success_response(Some(json!(9)), json!(1)).id,
        Some(json!(9))
    );
    assert_eq!(
        TestSession::error_response(Some(json!(2)), Error::Protocol("bad"))
            .error
            .unwrap()
            .code,
        -32600
    );
    assert_eq!(
        TestSession::rpc_error(Error::Io(std::io::Error::other("io"))).code,
        -32603
    );
    assert_eq!(
        TestSession::rpc_error(Error::invalid_params("bad")).code,
        -32602
    );

    session.stream = None;
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
