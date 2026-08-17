use super::Client;
use super::ConcurrentClient;
use crate::Error;
use serde_json::json;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, duplex};

#[tokio::test]
async fn call_sends_request_and_parses_result() {
    let (client_side, server_side) = duplex(512);
    let (mut server_read, mut server_write) = tokio::io::split(server_side);

    let server = tokio::spawn(async move {
        let mut request = Vec::new();
        let mut buf = [0_u8; 256];
        loop {
            let n = server_read.read(&mut buf).await.unwrap();
            if n == 0 {
                break;
            }
            request.extend_from_slice(&buf[..n]);
            if request.ends_with(b"\n") {
                break;
            }
        }

        assert_eq!(
            String::from_utf8(request).unwrap(),
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"sum\",\"params\":{\"a\":1}}\n"
        );

        server_write
            .write_all(br#"{"jsonrpc":"2.0","id":1,"result":{"total":2}}"#)
            .await
            .unwrap();
        server_write.write_all(b"\n").await.unwrap();
    });

    let mut client = Client::new(client_side);
    let value: serde_json::Value = client.call("sum", json!({"a": 1})).await.unwrap();
    assert_eq!(value, json!({"total": 2}));
    server.await.unwrap();
}

#[tokio::test]
async fn call_handles_protocol_and_server_errors() {
    let (client_side, server_side) = duplex(512);
    let (mut server_read, mut server_write) = tokio::io::split(server_side);

    let server = tokio::spawn(async move {
        let mut discard = Vec::new();
        let mut buf = [0_u8; 256];
        loop {
            let n = server_read.read(&mut buf).await.unwrap();
            if n == 0 {
                break;
            }
            discard.extend_from_slice(&buf[..n]);
            if discard.ends_with(b"\n") {
                break;
            }
        }

        server_write
            .write_all(br#"{"jsonrpc":"2.0","id":2,"result":null}"#)
            .await
            .unwrap();
        server_write.write_all(b"\n").await.unwrap();
    });

    let mut client = Client::new(client_side);
    assert!(matches!(
        client.call::<_, serde_json::Value>("sum", json!({})).await,
        Err(Error::Protocol("unexpected JSON-RPC id"))
    ));
    server.await.unwrap();

    let (client_side, server_side) = duplex(512);
    let (_server_read, mut server_write) = tokio::io::split(server_side);
    let server = tokio::spawn(async move {
        server_write
            .write_all(br#"{"jsonrpc":"2.0","id":1,"error":{"code":-32000,"message":"boom","data":{"retry":false}}}"#)
            .await
            .unwrap();
        server_write.write_all(b"\n").await.unwrap();
    });

    let mut client = Client::new(client_side);
    assert!(matches!(
        client.call::<_, serde_json::Value>("sum", json!({})).await,
        Err(Error::Rpc { message, .. }) if message == "boom"
    ));
    server.await.unwrap();
}

#[tokio::test]
async fn call_errors_when_response_is_missing() {
    let (client_side, server_side) = duplex(64);
    let server = tokio::spawn(async move {
        let mut server_side = server_side;
        let mut request = Vec::new();
        let mut buf = [0_u8; 64];
        loop {
            let n = server_side.read(&mut buf).await.unwrap();
            if n == 0 {
                break;
            }
            request.extend_from_slice(&buf[..n]);
            if request.ends_with(b"\n") {
                break;
            }
        }
    });
    let mut client = Client::new(client_side);

    let err = client
        .call::<_, serde_json::Value>("sum", json!({}))
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Protocol("no response")));
    server.await.unwrap();
}

#[tokio::test]
async fn call_preserves_custom_server_error_codes() {
    let (client_side, server_side) = duplex(512);
    let (_server_read, mut server_write) = tokio::io::split(server_side);
    let server = tokio::spawn(async move {
        server_write
            .write_all(
                br#"{"jsonrpc":"2.0","id":1,"error":{"code":-32001,"message":"custom boom"}}"#,
            )
            .await
            .unwrap();
        server_write.write_all(b"\n").await.unwrap();
    });

    let mut client = Client::new(client_side);
    assert!(matches!(
        client.call::<_, serde_json::Value>("sum", json!({})).await,
        Err(Error::Rpc {
            code: crate::ErrorCode::Custom(-32001),
            message,
            ..
        }) if message == "custom boom"
    ));
    server.await.unwrap();
}

#[tokio::test]
async fn call_skips_notifications_and_accepts_null_result() {
    let (client_side, server_side) = duplex(512);
    let (_server_read, mut server_write) = tokio::io::split(server_side);
    let server = tokio::spawn(async move {
        server_write
            .write_all(
                br#"{"jsonrpc":"2.0","method":"notice","params":{"ready":true}}
{"jsonrpc":"2.0","id":1,"result":null}
"#,
            )
            .await
            .unwrap();
    });

    let mut client = Client::new(client_side);
    let result: serde_json::Value = client.call("sum", json!({})).await.unwrap();
    assert_eq!(result, serde_json::Value::Null);
    server.await.unwrap();
}

#[tokio::test]
async fn call_rejects_invalid_response_version() {
    let (client_side, server_side) = duplex(512);
    let (_server_read, mut server_write) = tokio::io::split(server_side);
    let server = tokio::spawn(async move {
        server_write
            .write_all(
                br#"{"id":1,"result":true}
"#,
            )
            .await
            .unwrap();
    });

    let mut client = Client::new(client_side);
    assert!(matches!(
        client.call::<_, serde_json::Value>("sum", json!({})).await,
        Err(Error::Protocol("invalid JSON-RPC response version"))
    ));
    server.await.unwrap();
}

#[tokio::test]
async fn call_rejects_response_with_result_and_error() {
    let (client_side, server_side) = duplex(512);
    let (_server_read, mut server_write) = tokio::io::split(server_side);
    let server = tokio::spawn(async move {
        server_write
            .write_all(
                br#"{"jsonrpc":"2.0","id":1,"result":true,"error":{"code":-32000,"message":"boom"}}
"#,
            )
            .await
            .unwrap();
    });

    let mut client = Client::new(client_side);
    assert!(matches!(
        client.call::<_, serde_json::Value>("sum", json!({})).await,
        Err(Error::Protocol(
            "JSON-RPC response must contain exactly one result or error"
        ))
    ));
    server.await.unwrap();
}

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
