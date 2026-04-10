use crate::stream::ReaderWriter;
use crate::{Error, Handler};
use serde_json::{Value, json};
use std::sync::Arc;

struct EchoHandler;

#[async_trait::async_trait]
impl Handler for EchoHandler {
    async fn handle(&self, method: &str, params: Value) -> Result<Value, Error> {
        match method {
            "echo" => Ok(params),
            _ => Err(Error::method_not_found(method)),
        }
    }

    fn accepts_bidi(&self, method: &str) -> bool {
        method == "bidi"
    }

    async fn handle_bidi(
        &self,
        method: &str,
        params: Value,
        mut stream: ReaderWriter<Value, Value>,
    ) -> Result<(), Error> {
        if method != "bidi" {
            return Err(Error::method_not_found(method));
        }
        stream.write(json!({ "params": params })).await?;
        stream.close().await
    }
}

struct FailingHandler;

#[async_trait::async_trait]
impl Handler for FailingHandler {
    async fn handle(&self, _method: &str, _params: Value) -> Result<Value, Error> {
        Err(Error::Protocol("stop"))
    }
}

struct UnaryOnlyHandler;

#[async_trait::async_trait]
impl Handler for UnaryOnlyHandler {
    async fn handle(&self, method: &str, _params: Value) -> Result<Value, Error> {
        Err(Error::method_not_found(method))
    }
}

#[tokio::test]
async fn default_handle_bidi_reports_method_not_found() {
    let handler = EchoHandler;
    let (client, server) = tokio::io::duplex(128);
    let stream = crate::stream::open_bidi_server(server);

    let err = handler
        .handle_bidi("other", Value::Null, stream)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Rpc { .. }));

    drop(client);
}

#[tokio::test]
async fn arc_handler_delegates_all_methods() {
    let handler = Arc::new(EchoHandler);
    assert_eq!(
        handler.handle("echo", json!({"ok": true})).await.unwrap(),
        json!({"ok": true})
    );
    assert!(handler.accepts_bidi("bidi"));

    let (mut client, server) = tokio::io::duplex(128);
    let stream = crate::stream::open_bidi_server(server);
    handler
        .handle_bidi("bidi", json!({"ok": true}), stream)
        .await
        .unwrap();

    use tokio::io::AsyncReadExt;
    let mut buf = String::new();
    client.read_to_string(&mut buf).await.unwrap();
    assert_eq!(buf, "{\"params\":{\"ok\":true}}\n");
}

#[tokio::test]
async fn multi_handler_dispatches_and_bidi_routes() {
    let handler = super::super::handler::MultiHandler::new(vec![
        Box::new(EchoHandler),
        Box::new(FailingHandler),
    ]);

    assert_eq!(
        handler.handle("echo", json!({"n": 1})).await.unwrap(),
        json!({"n": 1})
    );

    let err = handler.handle("unknown", Value::Null).await.unwrap_err();
    assert!(matches!(err, Error::Protocol("stop")));

    let handler = super::super::handler::MultiHandler::new(vec![Box::new(EchoHandler)]);
    assert!(handler.accepts_bidi("bidi"));
    assert!(!handler.accepts_bidi("other"));

    let (mut client, server) = tokio::io::duplex(128);
    let bidi = crate::stream::open_bidi_server(server);
    handler
        .handle_bidi("bidi", json!({"x": 1}), bidi)
        .await
        .unwrap();

    use tokio::io::AsyncBufReadExt;
    let mut line = String::new();
    tokio::io::BufReader::new(&mut client)
        .read_line(&mut line)
        .await
        .unwrap();
    assert_eq!(line, "{\"params\":{\"x\":1}}\n");

    let err = handler
        .handle_bidi(
            "missing",
            Value::Null,
            crate::stream::open_bidi_server(client),
        )
        .await
        .unwrap_err();
    assert!(err.is_method_not_found());

    let handler = super::super::handler::MultiHandler::new(Vec::new());
    let err = handler.handle("missing", Value::Null).await.unwrap_err();
    assert!(err.is_method_not_found());
}

#[tokio::test]
async fn default_accepts_bidi_is_false() {
    let handler = UnaryOnlyHandler;
    assert!(!handler.accepts_bidi("anything"));
    let (_client, server) = tokio::io::duplex(64);
    let err = handler
        .handle_bidi(
            "missing",
            Value::Null,
            crate::stream::open_bidi_server(server),
        )
        .await
        .unwrap_err();
    assert!(err.is_method_not_found());
}
