use super::RpcClient;
use crate::Error;
use crate::Handler;
use crate::stream::ReaderWriter;
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::Mutex;

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

#[tokio::test]
async fn concurrent_calls_and_notifications_route_by_id() {
    let (rpc, mut notifications, _observed) = open_test_pair().await;
    let slow = tokio::spawn({
        let rpc = rpc.clone();
        async move { rpc.call::<_, String>("slow", json!({})).await }
    });
    let push = tokio::spawn({
        let rpc = rpc.clone();
        async move { rpc.call::<_, Value>("push", json!({})).await }
    });
    let sum = tokio::spawn({
        let rpc = rpc.clone();
        async move { rpc.call::<_, Value>("sum", json!({"a": 1, "b": 2})).await }
    });

    let pushed = tokio::time::timeout(Duration::from_secs(2), notifications.recv())
        .await
        .expect("pushed notification must arrive")
        .expect("notification channel must stay open");
    assert_eq!(pushed["method"], "pushed");
    assert_eq!(pushed["params"]["n"], 7);

    let total = sum.await.expect("join sum").expect("sum call");
    assert_eq!(total["total"], 3);
    let pushed_result = push.await.expect("join push").expect("push call");
    assert!(pushed_result.is_null());
    let slow_result = slow.await.expect("join slow").expect("slow call");
    assert_eq!(slow_result, "slow-done");
}

#[tokio::test]
async fn server_error_maps_to_rpc_error() {
    let (rpc, _notifications, _observed) = open_test_pair().await;
    let error = rpc
        .call::<_, Value>("fail", json!({}))
        .await
        .expect_err("fail call must error");
    assert!(
        matches!(error, Error::Rpc { ref message, .. } if message == "boom"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn request_times_out_when_peer_never_responds() {
    let (rpc, _notifications, _observed) =
        open_test_pair_with_timeout(Duration::from_millis(50)).await;
    let error = rpc
        .call::<_, Value>("never", json!({}))
        .await
        .expect_err("never call must time out");
    assert!(matches!(error, Error::RequestTimeout));
}

#[tokio::test]
async fn stream_close_fails_pending_requests_fast() {
    let (rpc, _notifications, _observed) = open_test_pair().await;
    let closing = tokio::spawn({
        let rpc = rpc.clone();
        async move { rpc.call::<_, Value>("close", json!({})).await }
    });
    let error = tokio::time::timeout(Duration::from_secs(2), closing)
        .await
        .expect("pending request must fail fast on stream close")
        .expect("close call task")
        .expect_err("close call must fail when the stream closes");
    assert!(
        matches!(error, Error::Protocol("rpc stream closed")),
        "unexpected error: {error}"
    );

    let error = rpc
        .call::<_, Value>("sum", json!({"a": 1, "b": 2}))
        .await
        .expect_err("call after close must fail");
    assert!(matches!(error, Error::Protocol("rpc client closed")));
    let error = rpc
        .notify("ping", json!({}))
        .await
        .expect_err("notify after close must fail");
    assert!(matches!(error, Error::Protocol("rpc client closed")));
}

#[tokio::test]
async fn notify_sends_fire_and_forget() {
    let (rpc, _notifications, observed) = open_test_pair().await;
    rpc.notify("ping", json!({"n": 3})).await.expect("notify");
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let seen = observed.lock().await;
            if seen.iter().any(|value| value["method"] == "ping") {
                break;
            }
            drop(seen);
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("server must observe the ping notification");
    let seen = observed.lock().await;
    let ping = seen
        .iter()
        .find(|value| value["method"] == "ping")
        .expect("ping notification");
    assert_eq!(ping["params"]["n"], 3);
}

#[tokio::test]
async fn batch_responses_route_out_of_order() {
    let (client_side, server_side) = tokio::io::duplex(64 * 1024);
    let server = tokio::spawn(async move {
        let (read_half, write_half) = tokio::io::split(server_side);
        let mut reader = tokio::io::BufReader::new(read_half);
        let mut writer = tokio::io::BufWriter::new(write_half);
        let codec = crate::codec::Codec::Json;
        let handshake: Value = codec
            .read(&mut reader)
            .await
            .expect("handshake")
            .expect("eof");
        assert_eq!(handshake["id"], 1);
        codec
            .write(
                &mut writer,
                &json!({"jsonrpc": "2.0", "id": 1, "result": null}),
            )
            .await
            .expect("handshake response");
        let first: Value = codec
            .read(&mut reader)
            .await
            .expect("first request")
            .expect("eof");
        let second: Value = codec
            .read(&mut reader)
            .await
            .expect("second request")
            .expect("eof");
        let id_a = first["id"].as_u64().expect("first request id");
        let id_b = second["id"].as_u64().expect("second request id");
        codec
            .write(
                &mut writer,
                &json!([
                    {"jsonrpc": "2.0", "id": id_b, "result": "b"},
                    {"jsonrpc": "2.0", "id": id_a, "result": "a"},
                ]),
            )
            .await
            .expect("batch response");
    });

    let session = crate::stream::open_bidi_client(client_side, "stream")
        .await
        .expect("open bidi stream");
    let (rpc, _notifications) = RpcClient::new(session);
    let rpc = Arc::new(rpc);
    let alpha = tokio::spawn({
        let rpc = rpc.clone();
        async move { rpc.call::<_, String>("alpha", json!({})).await }
    });
    let beta = tokio::spawn({
        let rpc = rpc.clone();
        async move { rpc.call::<_, String>("beta", json!({})).await }
    });

    assert_eq!(alpha.await.expect("join alpha").expect("alpha call"), "a");
    assert_eq!(beta.await.expect("join beta").expect("beta call"), "b");
    server.await.expect("server task");
}
