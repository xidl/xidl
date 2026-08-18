use super::super::ServerSession;
use super::SessionHandler;
use crate::Error;
use serde_json::{Value, json};
use tokio::io::AsyncReadExt;

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
async fn spawn_batch_reports_missing_stream() {
    let (_client, server) = tokio::io::duplex(64);
    let mut session = ServerSession::with_codec(server, SessionHandler, crate::codec::Codec::Json);
    session.writer = None;
    let items = vec![json!({"jsonrpc":"2.0","id":1,"method":"ok","params":null})];
    assert!(matches!(
        session.spawn_batch(items).await,
        Err(Error::Protocol("missing stream"))
    ));
}

#[tokio::test]
async fn handle_batch_empty_writes_protocol_error() {
    let (mut client, server) = tokio::io::duplex(512);
    let mut session = ServerSession::with_codec(server, SessionHandler, crate::codec::Codec::Json);
    assert!(session.handle_batch(vec![]).await.unwrap());
    drop(session);
    let mut output = String::new();
    client.read_to_string(&mut output).await.unwrap();
    let value: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
    assert_eq!(value["id"], json!(Value::Null));
    assert_eq!(value["error"]["code"], json!(-32600));
    assert_eq!(value["error"]["message"], json!("empty batch"));
}

#[tokio::test]
async fn handle_batch_with_writer_writes_responses() {
    let (mut client, server) = tokio::io::duplex(512);
    let mut session = ServerSession::with_codec(server, SessionHandler, crate::codec::Codec::Json);
    let items = vec![json!({"jsonrpc":"2.0","id":1,"method":"ok","params":{"a":2}})];
    assert!(session.handle_batch(items).await.unwrap());
    drop(session);
    let mut output = String::new();
    client.read_to_string(&mut output).await.unwrap();
    let value: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
    assert!(value.is_array());
    let items = value.as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], json!(1));
    assert_eq!(items[0]["result"], json!({ "echo": { "a": 2 } }));
}

#[tokio::test]
async fn process_batch_empty_returns_single_error() {
    let responses = ServerSession::<tokio::io::DuplexStream, SessionHandler>::process_batch(
        std::sync::Arc::new(SessionHandler),
        vec![],
    )
    .await;
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].error.as_ref().unwrap().code, -32600);
    assert_eq!(responses[0].error.as_ref().unwrap().message, "empty batch");
}
