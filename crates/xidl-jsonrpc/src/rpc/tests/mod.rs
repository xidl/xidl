use super::RpcClient;
use crate::Error;
use crate::Handler;
use crate::stream::ReaderWriter;
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::Mutex;

mod client;
mod handler;

/// Server handler that echoes sums, pushes notifications, and closes streams.
struct TestHandler {
    notifications: Arc<Mutex<Vec<Value>>>,
}

#[async_trait::async_trait]
impl Handler for TestHandler {
    async fn handle(&self, method: &str, _params: Value) -> Result<Value, Error> {
        Err(Error::method_not_found(method))
    }

    fn accepts_bidi(&self, _method: &str) -> bool {
        true
    }

    async fn handle_bidi(
        &self,
        _method: &str,
        _params: Value,
        stream: ReaderWriter<Value, Value>,
    ) -> Result<(), Error> {
        let (writer, mut reader) = stream.into_parts();
        let writer = Arc::new(Mutex::new(writer));
        loop {
            let Some(result) = reader.read().await else {
                break;
            };
            let value = result?;
            let Some(request_id) = value.get("id").cloned() else {
                self.notifications.lock().await.push(value);
                continue;
            };
            let Some(method) = value
                .get("method")
                .and_then(Value::as_str)
                .map(str::to_string)
            else {
                continue;
            };
            match method.as_str() {
                "sum" => {
                    let a = value["params"]["a"].as_i64().unwrap_or(0);
                    let b = value["params"]["b"].as_i64().unwrap_or(0);
                    let writer = writer.clone();
                    tokio::spawn(async move {
                        let mut writer = writer.lock().await;
                        let _ = writer
                            .write(json!({"jsonrpc":"2.0","id":request_id,"result":{"total":a+b}}))
                            .await;
                    });
                }
                "slow" => {
                    let writer = writer.clone();
                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        let mut writer = writer.lock().await;
                        let _ = writer
                            .write(json!({"jsonrpc":"2.0","id":request_id,"result":"slow-done"}))
                            .await;
                    });
                }
                "fail" => {
                    let writer = writer.clone();
                    tokio::spawn(async move {
                        let mut writer = writer.lock().await;
                        let _ = writer
                            .write(json!({"jsonrpc":"2.0","id":request_id,"error":{"code":-32000,"message":"boom"}}))
                            .await;
                    });
                }
                "push" => {
                    let writer = writer.clone();
                    tokio::spawn(async move {
                        let mut writer = writer.lock().await;
                        let _ = writer
                            .write(json!({"jsonrpc":"2.0","method":"pushed","params":{"n":7}}))
                            .await;
                        let _ = writer
                            .write(json!({"jsonrpc":"2.0","id":request_id,"result":null}))
                            .await;
                    });
                }
                "close" => break,
                "never" => {}
                _ => {}
            }
        }
        Ok(())
    }
}

fn unique_endpoint(label: &str) -> String {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    format!("rpc-client-{label}-{sequence}")
}

async fn open_test_pair() -> (
    Arc<RpcClient>,
    tokio::sync::mpsc::UnboundedReceiver<Value>,
    Arc<Mutex<Vec<Value>>>,
) {
    open_test_pair_with_timeout(Duration::from_secs(30)).await
}

async fn open_test_pair_with_timeout(
    request_timeout: Duration,
) -> (
    Arc<RpcClient>,
    tokio::sync::mpsc::UnboundedReceiver<Value>,
    Arc<Mutex<Vec<Value>>>,
) {
    let endpoint = unique_endpoint("pair");
    let uri = format!("inproc://{endpoint}");
    let notifications = Arc::new(Mutex::new(Vec::new()));
    let handler = TestHandler {
        notifications: notifications.clone(),
    };
    tokio::spawn(async move {
        let _ = crate::Server::builder()
            .with_service(handler)
            .serve_on(&uri)
            .await;
    });
    let stream = crate::connect_inproc(&endpoint).expect("connect inproc");
    let session = crate::stream::open_bidi_client(stream, "stream")
        .await
        .expect("open bidi stream");
    let (rpc, rx) = RpcClient::with_timeout(session, request_timeout);
    (Arc::new(rpc), rx, notifications)
}
