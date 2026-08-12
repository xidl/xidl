//! Batch request handling over the raw JSON-RPC wire.

use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn batch_with_multiple_valid_requests_returns_array_with_both_ids() {
    let (mut reader, mut writer) = open_raw_pair("batch-valid").await;
    write_json_line(
        &mut writer,
        &json!([
            { "jsonrpc": "2.0", "id": 1, "method": "echo", "params": { "a": 1 } },
            { "jsonrpc": "2.0", "id": 2, "method": "echo", "params": { "b": 2 } }
        ]),
    )
    .await
    .expect("write batch");

    let response = read_response(&mut reader).await;
    let items = response.as_array().expect("batch response is an array");
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["id"], json!(1));
    assert_eq!(items[0]["result"], json!({ "a": 1 }));
    assert_eq!(items[1]["id"], json!(2));
    assert_eq!(items[1]["result"], json!({ "b": 2 }));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn batch_with_mixed_items_reports_per_item_errors() {
    let (mut reader, mut writer) = open_raw_pair("batch-mixed").await;
    write_json_line(
        &mut writer,
        &json!([
            { "jsonrpc": "2.0", "id": 1, "method": "echo", "params": "ok" },
            { "jsonrpc": "2.0", "id": 2, "method": "nope", "params": null },
            { "jsonrpc": "1.0", "id": 3, "method": "echo", "params": "x" }
        ]),
    )
    .await
    .expect("write batch");

    let response = read_response(&mut reader).await;
    let items = response.as_array().expect("batch response is an array");
    assert_eq!(items.len(), 3);
    assert_eq!(items[0]["id"], json!(1));
    assert_eq!(items[0]["result"], json!("ok"));
    assert_eq!(items[1]["id"], json!(2));
    assert_eq!(items[1]["error"]["code"], json!(-32601));
    assert_eq!(items[2]["id"], json!(3));
    assert_eq!(items[2]["error"]["code"], json!(-32600));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn empty_batch_returns_invalid_request_error() {
    let (mut reader, mut writer) = open_raw_pair("batch-empty").await;
    write_json_line(&mut writer, &json!([]))
        .await
        .expect("write batch");

    let response = read_response(&mut reader).await;
    assert!(response.get("result").is_none());
    assert_eq!(response["id"], Value::Null);
    assert_eq!(response["error"]["code"], json!(-32600));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn batch_omits_responses_for_notifications() {
    let (mut reader, mut writer) = open_raw_pair("batch-notification").await;
    write_json_line(
        &mut writer,
        &json!([
            { "jsonrpc": "2.0", "method": "echo", "params": "ignored" },
            { "jsonrpc": "2.0", "id": 4, "method": "echo", "params": "kept" }
        ]),
    )
    .await
    .expect("write batch");

    let response = read_response(&mut reader).await;
    let items = response.as_array().expect("batch response is an array");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], json!(4));
    assert_eq!(items[0]["result"], json!("kept"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn notification_gets_no_response_and_connection_stays_usable() {
    let (mut reader, mut writer) = open_raw_pair("notification").await;
    write_json_line(
        &mut writer,
        &json!({ "jsonrpc": "2.0", "method": "echo", "params": "note" }),
    )
    .await
    .expect("write notification");
    write_json_line(
        &mut writer,
        &json!({ "jsonrpc": "2.0", "id": 9, "method": "echo", "params": "after" }),
    )
    .await
    .expect("write follow-up call");

    let response = read_response(&mut reader).await;
    assert_eq!(response["id"], json!(9));
    assert_eq!(response["result"], json!("after"));
    expect_silence(&mut reader).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn string_and_null_ids_are_echoed_verbatim() {
    let (mut reader, mut writer) = open_raw_pair("ids").await;

    write_json_line(
        &mut writer,
        &json!({ "jsonrpc": "2.0", "id": "abc", "method": "echo", "params": 1 }),
    )
    .await
    .expect("write string-id call");
    let response = read_response(&mut reader).await;
    assert_eq!(response["id"], json!("abc"));
    assert_eq!(response["result"], json!(1));

    write_json_line(
        &mut writer,
        &json!({ "jsonrpc": "2.0", "id": null, "method": "echo", "params": 2 }),
    )
    .await
    .expect("write null-id call");
    let response = read_response(&mut reader).await;
    assert_eq!(response["id"], Value::Null);
    assert_eq!(response["result"], json!(2));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn malformed_json_line_reports_parse_error_then_continues() {
    let (mut reader, mut writer) = open_raw_pair("malformed").await;
    writer
        .write_all(b"not-json\n")
        .await
        .expect("write malformed line");
    write_json_line(
        &mut writer,
        &json!({ "jsonrpc": "2.0", "id": 7, "method": "echo", "params": "ok" }),
    )
    .await
    .expect("write follow-up call");

    let first = read_response(&mut reader).await;
    assert_eq!(first["id"], Value::Null);
    assert_eq!(first["error"]["code"], json!(-32700));

    let second = read_response(&mut reader).await;
    assert_eq!(second["id"], json!(7));
    assert_eq!(second["result"], json!("ok"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn non_object_request_is_rejected_as_invalid_request() {
    let (mut reader, mut writer) = open_raw_pair("non-object").await;
    writer
        .write_all(b"42\n")
        .await
        .expect("write non-object request");

    let response = read_response(&mut reader).await;
    assert_eq!(response["id"], Value::Null);
    assert_eq!(response["error"]["code"], json!(-32600));
    assert!(
        response["error"]["message"]
            .as_str()
            .expect("error message is a string")
            .contains("object")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn missing_method_or_version_is_rejected_with_id_echoed() {
    let (mut reader, mut writer) = open_raw_pair("invalid-shapes").await;

    write_json_line(
        &mut writer,
        &json!({ "jsonrpc": "2.0", "id": 5, "params": null }),
    )
    .await
    .expect("write request without method");
    let response = read_response(&mut reader).await;
    assert_eq!(response["id"], json!(5));
    assert_eq!(response["error"]["code"], json!(-32600));
    assert!(
        response["error"]["message"]
            .as_str()
            .expect("error message is a string")
            .contains("method")
    );

    write_json_line(
        &mut writer,
        &json!({ "id": 6, "method": "echo", "params": null }),
    )
    .await
    .expect("write request without version");
    let response = read_response(&mut reader).await;
    assert_eq!(response["id"], json!(6));
    assert_eq!(response["error"]["code"], json!(-32600));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn large_batch_preserves_request_order() {
    let (mut reader, mut writer) = open_raw_pair("batch-large").await;
    let requests = (0..50)
        .map(|index| {
            json!({
                "jsonrpc": "2.0",
                "id": index,
                "method": "echo",
                "params": { "index": index },
            })
        })
        .collect::<Vec<_>>();
    write_json_line(&mut writer, &Value::Array(requests))
        .await
        .expect("write batch");

    let response = read_response(&mut reader).await;
    let items = response.as_array().expect("batch response is an array");
    assert_eq!(items.len(), 50);
    for (index, item) in items.iter().enumerate() {
        assert_eq!(item["id"], json!(index));
        assert_eq!(item["result"], json!({ "index": index }));
    }
}
