use super::super::RpcClient;
use super::{open_test_pair, open_test_pair_with_timeout};
use crate::Error;
use crate::stream::{ClientStreamWriter, Reader, ReaderWriter, boxed};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

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

#[tokio::test]
async fn write_failure_marks_closed_and_fails_pending() {
    // A session whose writer channel receiver is already gone: every write
    // fails with "stream writer is closed".
    let (tx, rx) = mpsc::channel::<Result<Value, Error>>(32);
    drop(rx);
    let response = tokio::spawn(async { Ok::<(), Error>(()) });
    let writer = ClientStreamWriter::new(tx, response);
    // The reader never produces data and never ends, so the read task cannot
    // mark the client closed before the write failure is observed.
    let reader = Reader::new(boxed(
        futures_util::stream::pending::<Result<Value, Error>>(),
    ));
    let session = ReaderWriter::new(writer, reader);
    let (rpc, _notifications) = RpcClient::new(session);

    let error = rpc
        .call::<_, Value>("sum", json!({}))
        .await
        .expect_err("write failure must surface");
    assert!(matches!(error, Error::Protocol("stream writer is closed")));

    // The failed write marked the client closed; later calls refuse quickly.
    let error = rpc
        .call::<_, Value>("mul", json!({}))
        .await
        .expect_err("call after write failure must refuse");
    assert!(matches!(error, Error::Protocol("rpc client closed")));
}

#[tokio::test]
async fn read_error_marks_closed_and_fails_pending() {
    let (tx, mut rx) = mpsc::channel::<Result<Value, Error>>(32);
    let (request_seen_tx, request_seen_rx) = oneshot::channel::<()>();
    let response = tokio::spawn(async move {
        // Consume the request so the call's write succeeds, then park forever.
        let _ = rx.recv().await;
        let _ = request_seen_tx.send(());
        std::future::pending::<()>().await;
        Ok::<(), Error>(())
    });
    let writer = ClientStreamWriter::new(tx, response);
    let (fail_tx, fail_rx) = oneshot::channel::<()>();
    let reader = Reader::new(boxed(futures_util::stream::once(async {
        // Wait until the pending call is registered, then fail the read stream
        // so the dispatch loop breaks and fails that call.
        fail_rx.await.expect("fail signal");
        Err::<Value, Error>(Error::Protocol("stream read failed"))
    })));
    let session = ReaderWriter::new(writer, reader);
    let (rpc, _notifications) = RpcClient::new(session);
    let rpc = Arc::new(rpc);

    let call = tokio::spawn({
        let rpc = rpc.clone();
        async move { rpc.call::<_, Value>("sum", json!({})).await }
    });
    // The pending entry is registered before the request is written, so once
    // the writer task observes the request, fail_all is guaranteed to fail it.
    request_seen_rx.await.expect("request seen");
    fail_tx.send(()).expect("fail signal");

    let error = call
        .await
        .expect("join call")
        .expect_err("read error must fail the pending call");
    assert!(matches!(error, Error::Protocol("rpc stream closed")));

    let error = rpc
        .call::<_, Value>("mul", json!({}))
        .await
        .expect_err("call after read error must refuse");
    assert!(matches!(error, Error::Protocol("rpc client closed")));
}
