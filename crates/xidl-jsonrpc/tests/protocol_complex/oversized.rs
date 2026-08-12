//! Oversized frame handling and frame-limit recovery.

use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn oversized_json_frame_is_rejected_and_server_recovers() {
    let (mut reader, mut writer) = open_raw_pair("oversized").await;
    let oversized = format!("{}\n", "x".repeat(4 * 1024 * 1024 + 64));
    writer
        .write_all(oversized.as_bytes())
        .await
        .expect("write oversized line");
    write_json_line(
        &mut writer,
        &json!({ "jsonrpc": "2.0", "id": 8, "method": "echo", "params": "ok" }),
    )
    .await
    .expect("write follow-up call");

    let first = read_response(&mut reader).await;
    assert_eq!(first["id"], Value::Null);
    assert_eq!(first["error"]["code"], json!(-32700));
    let message = first["error"]["message"].as_str().expect("error message");
    assert!(
        message.contains("frame exceeds maximum length") && message.contains("4194304"),
        "oversized frame must report the frame limit, got: {message:?}"
    );

    let second = read_response(&mut reader).await;
    assert_eq!(second["id"], json!(8));
    assert_eq!(second["result"], json!("ok"));
}

/// Builds a valid JSON-RPC echo request whose wire line is exactly `len` bytes
/// (excluding the newline), padding `params` with `x` characters. Returns the
/// line and the number of padding characters it carries.
fn json_line_of_exact_len(len: usize) -> (String, usize) {
    let head = r#"{"jsonrpc":"2.0","id":1,"method":"echo","params":""#;
    let tail = r#""}"#;
    let content = len - head.len() - tail.len();
    assert!(content > 0, "target length {len} leaves no params padding");
    let mut line = String::with_capacity(len);
    line.push_str(head);
    line.push_str(&"x".repeat(content));
    line.push_str(tail);
    assert_eq!(line.len(), len);
    (line, content)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exact_max_json_frame_is_accepted() {
    let (mut reader, mut writer) = open_raw_pair("exact-max").await;
    let max = 4 * 1024 * 1024;
    let (mut exact, padding) = json_line_of_exact_len(max);
    exact.push('\n');
    writer
        .write_all(exact.as_bytes())
        .await
        .expect("write exact-max line");
    write_json_line(
        &mut writer,
        &json!({ "jsonrpc": "2.0", "id": 9, "method": "echo", "params": "ok" }),
    )
    .await
    .expect("write follow-up call");

    let first = read_response(&mut reader).await;
    assert_eq!(first["id"], json!(1));
    assert_eq!(
        first["result"].as_str().map(str::len),
        Some(padding),
        "params padding must round-trip in full"
    );

    let second = read_response(&mut reader).await;
    assert_eq!(second["id"], json!(9));
    assert_eq!(second["result"], json!("ok"));
}
