use super::{read_json_line, write_json_line};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};

#[tokio::test]
async fn write_json_line_serializes_and_terminates_with_newline() {
    let (mut writer, mut reader) = tokio::io::duplex(128);
    let task = tokio::spawn(async move {
        write_json_line(&mut writer, &json!({"ok": true}))
            .await
            .unwrap();
    });

    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).await.unwrap();
    task.await.unwrap();

    assert_eq!(String::from_utf8(bytes).unwrap(), "{\"ok\":true}\n");
}

#[tokio::test]
async fn read_json_line_handles_success_eof_and_parse_errors() {
    let (mut writer, reader) = tokio::io::duplex(128);
    writer.write_all(br#"{"value":1}"#).await.unwrap();
    writer.write_all(b"\n").await.unwrap();
    writer.shutdown().await.unwrap();

    let mut reader = BufReader::new(reader);
    let first = read_json_line::<_, serde_json::Value>(&mut reader)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(first, json!({"value": 1}));
    assert!(
        read_json_line::<_, serde_json::Value>(&mut reader)
            .await
            .unwrap()
            .is_none()
    );

    let (mut writer, reader) = tokio::io::duplex(128);
    writer.write_all(b"not-json\n").await.unwrap();
    writer.shutdown().await.unwrap();
    let mut reader = BufReader::new(reader);
    assert!(
        read_json_line::<_, serde_json::Value>(&mut reader)
            .await
            .is_err()
    );
}
