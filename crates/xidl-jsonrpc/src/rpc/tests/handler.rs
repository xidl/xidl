use super::TestHandler;
use crate::{Error, ErrorCode, Handler};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::test]
async fn handler_handle_reports_method_not_found() {
    let handler = TestHandler {
        notifications: Arc::new(Mutex::new(Vec::new())),
    };
    let error = handler.handle("unknown", json!({})).await.unwrap_err();
    assert!(matches!(
        error,
        Error::Rpc {
            code: ErrorCode::MethodNotFound,
            ..
        }
    ));
}
