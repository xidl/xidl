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
    let mut session = ServerSession::new(server, SessionHandler);

    let task = tokio::spawn(async move { session.run().await.unwrap() });
    client
        .write_all(br#"{"id":1,"method":"ok","params":{"a":1}}"#)
        .await
        .unwrap();
    client.write_all(b"\n").await.unwrap();
    client
        .write_all(br#"{"id":2,"method":"rpc","params":null}"#)
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
    let mut session = ServerSession::new(server, SessionHandler);
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
    assert_eq!(responses[1]["error"]["message"], json!("missing method"));
}

#[tokio::test]
async fn bidi_requests_take_over_the_stream() {
    let (mut client, server) = tokio::io::duplex(512);
    let mut session = ServerSession::new(server, SessionHandler);
    let task = tokio::spawn(async move { session.run().await.unwrap() });

    client
        .write_all(br#"{"id":1,"method":"bidi","params":{"n":7}}"#)
        .await
        .unwrap();
    client.write_all(b"\n").await.unwrap();
    client.shutdown().await.unwrap();

    let mut output = String::new();
    client.read_to_string(&mut output).await.unwrap();
    task.await.unwrap();

    assert_eq!(output, "{\"stream\":{\"n\":7}}\n");
}

#[tokio::test]
async fn private_helpers_map_errors_and_missing_streams() {
    let (_client, server) = tokio::io::duplex(64);
    let mut session = ServerSession::new(server, SessionHandler);
    type TestSession = ServerSession<tokio::io::DuplexStream, SessionHandler>;

    assert_eq!(TestSession::success_response(Some(9), json!(1)).id, Some(9));
    assert_eq!(
        TestSession::error_response(Some(2), Error::Protocol("bad"))
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
        session.write_result(Some(1), Value::Null).await,
        Err(Error::Protocol("missing stream"))
    ));
    assert!(matches!(
        session.handle_line(r#"{"id":1,"method":"bidi"}"#).await,
        Err(Error::Protocol("missing stream"))
    ));
    session.run().await.unwrap();
}
