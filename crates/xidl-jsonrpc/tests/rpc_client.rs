//! End-to-end integration tests for the concurrent [`RpcClient`].
//!
//! These tests exercise `RpcClient` through the public `Server`, `Handler`,
//! `stream`, and `transport` APIs: concurrent request multiplexing, server
//! notifications, per-request timeouts, fail-fast on stream close, and
//! out-of-order batch responses.

#![cfg(feature = "tokio")]

use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use xidl_jsonrpc::stream::{ClientStreamWriter, ReaderWriter};
use xidl_jsonrpc::{Error, Handler, RpcClient, Server};

fn random_endpoint(prefix: &str) -> String {
    static SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    static STARTED: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    let started = *STARTED.get_or_init(std::time::Instant::now);
    let elapsed = started.elapsed().as_nanos();
    let sequence = SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("{prefix}-{elapsed}-{sequence}")
}

type ServerTask = tokio::task::JoinHandle<Result<(), Error>>;

/// Server handler for the `rpc-stream` bidirectional method.
struct RpcStreamHandler {
    notifications: Arc<std::sync::Mutex<Vec<Value>>>,
}

#[async_trait::async_trait]
impl Handler for RpcStreamHandler {
    async fn handle(&self, method: &str, _params: Value) -> Result<Value, Error> {
        Err(Error::method_not_found(method))
    }

    fn accepts_bidi(&self, method: &str) -> bool {
        method == "rpc-stream"
    }

    async fn handle_bidi(
        &self,
        method: &str,
        _params: Value,
        stream: ReaderWriter<Value, Value>,
    ) -> Result<(), Error> {
        if method != "rpc-stream" {
            return Err(Error::method_not_found(method));
        }
        let (writer, mut reader) = stream.into_parts();
        let writer: Arc<tokio::sync::Mutex<ClientStreamWriter<Value, ()>>> =
            Arc::new(tokio::sync::Mutex::new(writer));
        loop {
            let Some(result) = reader.read().await else {
                break;
            };
            let value = result?;
            let Some(request_id) = value.get("id").cloned() else {
                self.notifications
                    .lock()
                    .expect("notification lock")
                    .push(value);
                continue;
            };
            let Some(method) = value.get("method").and_then(Value::as_str) else {
                continue;
            };
            match method {
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
                "fail" => {
                    let writer = writer.clone();
                    tokio::spawn(async move {
                        let mut writer = writer.lock().await;
                        let _ = writer
                            .write(json!({"jsonrpc":"2.0","id":request_id,"error":{"code":-32000,"message":"boom"}}))
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

/// Spawns an inproc server and returns a connected `RpcClient` plus the
/// notification receiver and the notifications the server observed.
async fn open_rpc_client(
    request_timeout: Duration,
) -> (
    Arc<RpcClient>,
    mpsc::UnboundedReceiver<Value>,
    Arc<std::sync::Mutex<Vec<Value>>>,
    ServerTask,
) {
    let endpoint = random_endpoint("rpc-client");
    let uri = format!("inproc://{endpoint}");
    let notifications = Arc::new(std::sync::Mutex::new(Vec::new()));
    let handler = RpcStreamHandler {
        notifications: notifications.clone(),
    };
    let server =
        tokio::spawn(async move { Server::builder().with_service(handler).serve_on(&uri).await });
    let stream = xidl_jsonrpc::connect_inproc(&endpoint).expect("connect inproc");
    let session = xidl_jsonrpc::stream::open_bidi_client(stream, "rpc-stream")
        .await
        .expect("open rpc stream");
    let (rpc, rx) = RpcClient::with_timeout(session, request_timeout);
    (Arc::new(rpc), rx, notifications, server)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_client_multiplexes_requests_and_delivers_notifications() {
    let (rpc, mut notifications, _observed, server) =
        open_rpc_client(Duration::from_secs(30)).await;

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

    let pushed = tokio::time::timeout(Duration::from_secs(10), notifications.recv())
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

    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_client_maps_server_errors() {
    let (rpc, _notifications, _observed, server) = open_rpc_client(Duration::from_secs(30)).await;

    let error = rpc
        .call::<_, Value>("fail", json!({}))
        .await
        .expect_err("fail call must error");
    assert!(
        matches!(error, Error::Rpc { ref message, .. } if message == "boom"),
        "unexpected error: {error}"
    );

    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_client_pending_requests_fail_fast_when_stream_closes() {
    let (rpc, _notifications, _observed, server) = open_rpc_client(Duration::from_secs(30)).await;

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

    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_client_times_out_when_peer_never_responds() {
    let (rpc, _notifications, _observed, server) = open_rpc_client(Duration::from_millis(50)).await;

    let error = rpc
        .call::<_, Value>("never", json!({}))
        .await
        .expect_err("never call must time out");
    assert!(matches!(error, Error::RequestTimeout));

    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_client_notify_reaches_the_server() {
    let (rpc, _notifications, observed, server) = open_rpc_client(Duration::from_secs(30)).await;

    rpc.notify("ping", json!({"n": 3})).await.expect("notify");

    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let ping_seen = {
                let seen = observed.lock().expect("notification lock");
                seen.iter().any(|value| value["method"] == "ping")
            };
            if ping_seen {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("server must observe the ping notification");

    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_client_routes_out_of_order_batch_responses() {
    let (client_side, server_side) = tokio::io::duplex(64 * 1024);
    let server = tokio::spawn(async move {
        let mut bidi = xidl_jsonrpc::stream::open_bidi_server(server_side);
        let handshake = bidi.read().await.expect("handshake expected")?;
        assert_eq!(handshake["id"], 1);
        assert_eq!(handshake["method"], "rpc-stream");
        bidi.write(json!({"jsonrpc":"2.0","id":1,"result":null}))
            .await?;
        // The two concurrent client calls may be written in either order, so
        // respond to each by method and reverse the arrival order to prove
        // that the client routes responses by id rather than position.
        let first = bidi.read().await.expect("first request expected")?;
        let second = bidi.read().await.expect("second request expected")?;
        let first_method = first["method"].as_str().expect("first method");
        let second_method = second["method"].as_str().expect("second method");
        bidi.write(json!([
            {"jsonrpc":"2.0","id":second["id"],"result":second_method},
            {"jsonrpc":"2.0","id":first["id"],"result":first_method},
        ]))
        .await?;
        bidi.close().await
    });

    let session = xidl_jsonrpc::stream::open_bidi_client(client_side, "rpc-stream")
        .await
        .expect("open rpc stream");
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

    assert_eq!(
        alpha.await.expect("join alpha").expect("alpha call"),
        "alpha"
    );
    assert_eq!(beta.await.expect("join beta").expect("beta call"), "beta");
    server
        .await
        .expect("join server")
        .expect("server task failed");
}
