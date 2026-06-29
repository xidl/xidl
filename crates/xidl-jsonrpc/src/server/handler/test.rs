use super::{Handler, MultiHandler};
use crate::Error;
use serde_json::Value;
use std::sync::Arc;

struct UnaryOnly;

#[async_trait::async_trait]
impl Handler for UnaryOnly {
    async fn handle(&self, method: &str, _params: Value) -> Result<Value, Error> {
        Err(Error::method_not_found(method))
    }
}

struct BidiOnly;

#[async_trait::async_trait]
impl Handler for BidiOnly {
    async fn handle(&self, method: &str, _params: Value) -> Result<Value, Error> {
        Err(Error::method_not_found(method))
    }

    fn accepts_bidi(&self, method: &str) -> bool {
        method == "bidi"
    }

    async fn handle_bidi(
        &self,
        _method: &str,
        _params: Value,
        stream: crate::stream::ReaderWriter<Value, Value>,
    ) -> Result<(), Error> {
        stream.cancel().await
    }
}

#[tokio::test]
async fn default_and_arc_handler_paths_are_covered() {
    let handler = UnaryOnly;
    assert!(!handler.accepts_bidi("missing"));
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

    let arc = Arc::new(BidiOnly);
    let (_client, server) = tokio::io::duplex(64);
    arc.handle("other", Value::Null).await.unwrap_err();
    arc.handle_bidi("bidi", Value::Null, crate::stream::open_bidi_server(server))
        .await
        .unwrap();
}

#[tokio::test]
async fn empty_multi_handler_reports_method_not_found() {
    let handler = MultiHandler::new(Vec::new());
    let err = handler.handle("missing", Value::Null).await.unwrap_err();
    assert!(err.is_method_not_found());
}
