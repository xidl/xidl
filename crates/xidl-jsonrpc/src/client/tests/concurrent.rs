use super::super::ConcurrentClient;
use super::{FailingWriteStream, wait_for_written};
use crate::Error;
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, duplex};

#[tokio::test]
async fn concurrent_client_multiplexes_calls_and_notifications() {
    let (client_side, server_side) = duplex(1024);
    let (server_read, mut server_write) = tokio::io::split(server_side);
    let server = tokio::spawn(async move {
        let mut server_read = BufReader::new(server_read);
        let mut line = Vec::new();
        server_read.read_until(b'\n', &mut line).await.unwrap();
        assert!(String::from_utf8_lossy(&line).contains("\"method\":\"sum\""));
        server_write
            .write_all(
                br#"{"jsonrpc":"2.0","id":1,"result":{"total":2}}
"#,
            )
            .await
            .unwrap();

        line.clear();
        server_read.read_until(b'\n', &mut line).await.unwrap();
        assert!(String::from_utf8_lossy(&line).contains("\"method\":\"notice\""));
    });

    let client = ConcurrentClient::new(client_side);
    let result: serde_json::Value = client.call("sum", json!({"a": 1})).await.unwrap();
    assert_eq!(result, json!({"total": 2}));
    client
        .notify("notice", json!({"ready": true}))
        .await
        .unwrap();
    server.await.unwrap();

    for _ in 0..16 {
        if client.closed.load(std::sync::atomic::Ordering::Acquire) {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(client.closed.load(std::sync::atomic::Ordering::Acquire));
    assert!(matches!(
        client
            .call::<_, serde_json::Value>("after-close", json!({}))
            .await,
        Err(Error::Protocol("rpc client closed"))
    ));
    assert!(matches!(
        client.notify("after-close", json!({})).await,
        Err(Error::Protocol("rpc client closed"))
    ));
}

#[cfg(feature = "msgpack")]
#[tokio::test]
async fn concurrent_client_msgpack_roundtrips_a_call() {
    let (client_side, server_side) = duplex(1024);
    let (server_read, mut server_write) = tokio::io::split(server_side);
    let server = tokio::spawn(async move {
        let mut server_read = BufReader::new(server_read);
        let request: serde_json::Value = crate::codec::Codec::Msgpack
            .read(&mut server_read)
            .await
            .unwrap()
            .expect("server must receive the msgpack request");
        assert!(request.get("method").is_some());
        crate::codec::Codec::Msgpack
            .write(
                &mut server_write,
                &json!({"jsonrpc": "2.0", "id": 1, "result": {"ok": true}}),
            )
            .await
            .unwrap();
    });

    let client = ConcurrentClient::new_msgpack(client_side);
    let result: serde_json::Value = client.call("sum", json!({"a": 1})).await.unwrap();
    assert_eq!(result, json!({"ok": true}));
    server.await.unwrap();
}

#[tokio::test]
async fn concurrent_client_call_write_failure_marks_closed_and_fails_pending() {
    let written = Arc::new(Mutex::new(Vec::new()));
    let fail_writes = Arc::new(AtomicBool::new(false));
    let stream = FailingWriteStream {
        written: written.clone(),
        fail_writes: fail_writes.clone(),
    };
    let client = Arc::new(ConcurrentClient::new(stream));

    // First call registers a pending request and writes successfully.
    let call_client = client.clone();
    let call1 = tokio::spawn(async move {
        call_client
            .call::<_, serde_json::Value>("sum", json!({}))
            .await
    });
    wait_for_written(&written, b"sum").await;

    // Subsequent writes now fail, which also fails the pending call.
    fail_writes.store(true, Ordering::Release);
    let result = client.call::<_, serde_json::Value>("mul", json!({})).await;
    assert!(matches!(result, Err(Error::Io(_))));
    assert!(client.closed.load(Ordering::Acquire));

    let call1_result = call1.await.unwrap();
    assert!(matches!(
        call1_result,
        Err(Error::Protocol("rpc client write failed"))
    ));
}

#[tokio::test]
async fn concurrent_client_routes_server_error_responses() {
    let (client_side, server_side) = duplex(1024);
    let (server_read, mut server_write) = tokio::io::split(server_side);
    let server = tokio::spawn(async move {
        let mut line = Vec::new();
        let mut server_read = BufReader::new(server_read);
        server_read.read_until(b'\n', &mut line).await.unwrap();
        server_write
            .write_all(br#"{"jsonrpc":"2.0","id":1,"error":{"code":-32000,"message":"boom"}}"#)
            .await
            .unwrap();
        server_write.write_all(b"\n").await.unwrap();
    });

    let client = ConcurrentClient::new(client_side);
    assert!(matches!(
        client.call::<_, serde_json::Value>("sum", json!({})).await,
        Err(Error::Rpc { message, .. }) if message == "boom"
    ));
    server.await.unwrap();
}

#[tokio::test]
async fn concurrent_client_notify_write_failure_marks_closed() {
    let written = Arc::new(Mutex::new(Vec::new()));
    let fail_writes = Arc::new(AtomicBool::new(false));
    let stream = FailingWriteStream {
        written: written.clone(),
        fail_writes: fail_writes.clone(),
    };
    let client = ConcurrentClient::new(stream);

    // A successful notification exercises the success branch of notify's
    // write check.
    client
        .notify("notice", json!({"ready": true}))
        .await
        .unwrap();

    fail_writes.store(true, Ordering::Release);
    assert!(
        client
            .notify("notice", json!({"ready": false}))
            .await
            .is_err()
    );
    assert!(client.closed.load(Ordering::Acquire));
}

#[tokio::test]
async fn concurrent_client_routes_batch_responses_and_ignores_unmatched_entries() {
    let (client_side, server_side) = duplex(2048);
    let (server_read, mut server_write) = tokio::io::split(server_side);
    let server = tokio::spawn(async move {
        let mut line = Vec::new();
        let mut server_read = BufReader::new(server_read);
        server_read.read_until(b'\n', &mut line).await.unwrap();
        server_write
            .write_all(
                br#"[{"jsonrpc":"2.0","id":1,"result":{"ok":true}},{"jsonrpc":"2.0","method":"notice"},{"jsonrpc":"2.0","id":99,"error":{"code":-32000,"message":"stale"}}]
"#,
            )
            .await
            .unwrap();
    });

    let client = std::sync::Arc::new(ConcurrentClient::new(client_side));
    let call_client = client.clone();
    let handle = tokio::spawn(async move {
        call_client
            .call::<_, serde_json::Value>("sum", json!({}))
            .await
    });
    let result = handle.await.unwrap().unwrap();
    assert_eq!(result, json!({"ok": true}));
    server.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn concurrent_client_times_out_when_response_never_arrives() {
    let (client_side, server_side) = duplex(1024);
    let (server_read, _server_write) = tokio::io::split(server_side);
    let server = tokio::spawn(async move {
        let mut line = Vec::new();
        let mut server_read = BufReader::new(server_read);
        server_read.read_until(b'\n', &mut line).await.unwrap();
        std::future::pending::<()>().await;
    });

    let client = std::sync::Arc::new(ConcurrentClient::new(client_side));
    let call_client = client.clone();
    let handle = tokio::spawn(async move {
        call_client
            .call::<_, serde_json::Value>("sum", json!({}))
            .await
    });
    tokio::task::yield_now().await;
    tokio::task::yield_now().await;
    tokio::time::advance(std::time::Duration::from_secs(31)).await;

    let result = handle.await.unwrap();
    assert!(matches!(result, Err(Error::RequestTimeout)));
    // The server task deliberately never returns (it holds the stream open
    // so the client times out instead of seeing EOF), so detach it.
    drop(server);
}
