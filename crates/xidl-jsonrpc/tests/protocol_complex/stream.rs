//! Bidi stream handling over the raw JSON-RPC wire.

use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bidi_in_batch_is_rejected_per_item_and_connection_survives() {
    let uri = format!("inproc://{}", random_endpoint("batch-bidi"));
    let serve_uri = uri.clone();
    let server = tokio::spawn(async move {
        Server::builder()
            .with_service(EchoBidiHandler)
            .serve_on(&serve_uri)
            .await
    });
    let stream = connect_with_retry(&uri).await.expect("connect inproc");
    let (read_half, mut write_half) = tokio::io::split(stream);
    let mut reader = BufReader::new(read_half);

    write_json_line(
        &mut write_half,
        &json!([
            { "jsonrpc": "2.0", "id": 1, "method": "bidi", "params": null },
            { "jsonrpc": "2.0", "id": 2, "method": "echo", "params": "ok" }
        ]),
    )
    .await
    .expect("write batch");

    let response = read_response(&mut reader).await;
    let items = response.as_array().expect("batch response is an array");
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["id"], json!(1));
    assert_eq!(items[0]["error"]["code"], json!(-32600));
    assert!(
        items[0]["error"]["message"]
            .as_str()
            .expect("error message is a string")
            .contains("batch")
    );
    assert_eq!(items[1]["id"], json!(2));
    assert_eq!(items[1]["result"], json!("ok"));

    write_json_line(
        &mut write_half,
        &json!({ "jsonrpc": "2.0", "id": 3, "method": "echo", "params": "again" }),
    )
    .await
    .expect("write follow-up call");
    let response = read_response(&mut reader).await;
    assert_eq!(response["id"], json!(3));
    assert_eq!(response["result"], json!("again"));

    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bidi_with_id_streams_items_after_handshake() {
    let uri = format!("inproc://{}", random_endpoint("bidi-ack"));
    let serve_uri = uri.clone();
    let server = tokio::spawn(async move {
        Server::builder()
            .with_service(EchoBidiHandler)
            .serve_on(&serve_uri)
            .await
    });

    let stream = connect_with_retry(&uri).await.expect("connect inproc");
    let mut bidi = xidl_jsonrpc::stream::open_bidi_client(stream, "bidi")
        .await
        .expect("open bidi stream");

    // open_bidi_client already consumed the server's handshake
    // acknowledgement (result null, id 1), so the first value read is the
    // first echoed stream item.
    bidi.write(json!({ "n": 1 }))
        .await
        .expect("write stream item");
    let echoed = bidi
        .read()
        .await
        .expect("echo expected")
        .expect("echo read failed");
    assert_eq!(echoed, json!({ "n": 1 }));

    bidi.close().await.expect("close bidi stream");
    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn id_less_bidi_request_streams_without_ack() {
    let uri = format!("inproc://{}", random_endpoint("bidi-idless"));
    let serve_uri = uri.clone();
    let server = tokio::spawn(async move {
        Server::builder()
            .with_service(EchoBidiHandler)
            .serve_on(&serve_uri)
            .await
    });
    let stream = connect_with_retry(&uri).await.expect("connect inproc");
    let (read_half, mut write_half) = tokio::io::split(stream);
    let mut reader = BufReader::new(read_half);

    // Notification-style bidi request: no id, so the server must not emit an
    // ack and must switch straight to the stream.
    write_json_line(
        &mut write_half,
        &json!({ "jsonrpc": "2.0", "method": "bidi", "params": null }),
    )
    .await
    .expect("write id-less bidi request");
    write_json_line(&mut write_half, &json!({ "n": 2 }))
        .await
        .expect("write stream item");
    write_half.shutdown().await.expect("shutdown write half");

    let echoed = read_response(&mut reader).await;
    assert_eq!(echoed, json!({ "n": 2 }));

    // After the client shuts down its write half, the server's bidi handler
    // sees EOF and closes the connection: the client must observe EOF, not a
    // further frame.
    let mut line = String::new();
    let bytes = tokio::time::timeout(Duration::from_secs(10), reader.read_line(&mut line))
        .await
        .expect("timed out waiting for the server to close the stream")
        .expect("failed to read after shutdown");
    assert_eq!(bytes, 0, "expected stream close, got: {line:?}");
    server.abort();
}
