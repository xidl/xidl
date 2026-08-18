use super::super::{ServerSession, response::ResponseCodec};
use super::{ErrorReader, SessionHandler};
use crate::{Error, Handler};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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

struct RejectingBidiHandler;

#[async_trait::async_trait]
impl Handler for RejectingBidiHandler {
    async fn handle(&self, _method: &str, _params: Value) -> Result<Value, Error> {
        Ok(Value::Null)
    }
    fn accepts_bidi(&self, method: &str) -> bool {
        method == "bidi"
    }
    async fn validate_bidi(&self, _method: &str, _params: &Value) -> Result<(), Error> {
        Err(Error::invalid_params("bidi rejected"))
    }
}

#[tokio::test]
async fn bidi_validation_failure_reports_error() {
    let (mut client, server) = tokio::io::duplex(512);
    let mut session =
        ServerSession::with_codec(server, RejectingBidiHandler, crate::codec::Codec::Json);
    let request = super::super::ParsedRequest {
        id: Some(json!(5)),
        method: "bidi".to_string(),
        params: Value::Null,
    };
    assert!(session.handle_request(request).await.unwrap());
    drop(session);
    let mut output = String::new();
    client.read_to_string(&mut output).await.unwrap();
    let value: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
    assert_eq!(value["id"], json!(5));
    assert_eq!(value["error"]["code"], json!(-32602));
    assert_eq!(value["error"]["message"], json!("bidi rejected"));
}

#[tokio::test]
async fn handle_request_missing_stream_non_bidi() {
    let (_client, server) = tokio::io::duplex(64);
    let mut session = ServerSession::with_codec(server, SessionHandler, crate::codec::Codec::Json);
    session.writer = None;
    let request = super::super::ParsedRequest {
        id: Some(json!(1)),
        method: "ok".to_string(),
        params: Value::Null,
    };
    assert!(matches!(
        session.handle_request(request).await,
        Err(Error::Protocol("missing stream"))
    ));
}

#[tokio::test]
async fn request_timeout_maps_to_server_error() {
    let response = ResponseCodec::rpc_error(Error::RequestTimeout);
    assert_eq!(response.code, -32000);
    assert_eq!(response.message, "internal server error");
}

#[tokio::test]
async fn error_reader_async_write_impls_are_drivable() {
    let mut error_reader = ErrorReader;
    let write_result = error_reader.write(b"data").await;
    assert!(matches!(write_result, Ok(0)));
    let flush_result = error_reader.flush().await;
    assert!(flush_result.is_ok());
    let shutdown_result = error_reader.shutdown().await;
    assert!(shutdown_result.is_ok());
}
