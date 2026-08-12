use super::Codec;
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};

#[tokio::test]
async fn json_codec_serializes_and_terminates_with_newline() {
    let (mut writer, mut reader) = tokio::io::duplex(128);
    let task = tokio::spawn(async move {
        Codec::Json
            .write(&mut writer, &json!({"ok": true}))
            .await
            .unwrap();
    });

    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).await.unwrap();
    task.await.unwrap();

    assert_eq!(String::from_utf8(bytes).unwrap(), "{\"ok\":true}\n");
}

#[tokio::test]
async fn json_codec_handles_success_eof_and_parse_errors() {
    let (mut writer, reader) = tokio::io::duplex(128);
    writer.write_all(br#"{"value":1}"#).await.unwrap();
    writer.write_all(b"\n").await.unwrap();
    writer.shutdown().await.unwrap();

    let mut reader = BufReader::new(reader);
    let first = Codec::Json
        .read::<_, serde_json::Value>(&mut reader)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(first, json!({"value": 1}));
    assert!(
        Codec::Json
            .read::<_, serde_json::Value>(&mut reader)
            .await
            .unwrap()
            .is_none()
    );

    let (mut writer, reader) = tokio::io::duplex(128);
    writer.write_all(b"not-json\n").await.unwrap();
    writer.shutdown().await.unwrap();
    let mut reader = BufReader::new(reader);
    assert!(
        Codec::Json
            .read::<_, serde_json::Value>(&mut reader)
            .await
            .is_err()
    );
}

#[cfg(feature = "msgpack")]
#[tokio::test]
async fn msgpack_codec_roundtrips_with_length_prefix() {
    let (mut writer, mut reader) = tokio::io::duplex(256);
    let task = tokio::spawn(async move {
        Codec::Msgpack
            .write(&mut writer, &json!({"ok": true, "n": 7}))
            .await
            .unwrap();
    });

    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).await.unwrap();
    task.await.unwrap();

    // 4-byte big-endian length prefix followed by the msgpack payload.
    let len = u32::from_be_bytes(bytes[0..4].try_into().unwrap()) as usize;
    assert_eq!(len, bytes.len() - 4);
    let value: serde_json::Value = rmp_serde::from_slice(&bytes[4..]).unwrap();
    assert_eq!(value, json!({"ok": true, "n": 7}));
}

#[cfg(feature = "msgpack")]
#[tokio::test]
async fn msgpack_codec_reads_frames_and_reports_eof() {
    let (mut writer, reader) = tokio::io::duplex(256);
    let payload = rmp_serde::to_vec(&json!({"value": 1})).unwrap();
    writer
        .write_all(&(payload.len() as u32).to_be_bytes())
        .await
        .unwrap();
    writer.write_all(&payload).await.unwrap();
    writer.shutdown().await.unwrap();

    let mut reader = BufReader::new(reader);
    let first = Codec::Msgpack
        .read::<_, serde_json::Value>(&mut reader)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(first, json!({"value": 1}));
    assert!(
        Codec::Msgpack
            .read::<_, serde_json::Value>(&mut reader)
            .await
            .unwrap()
            .is_none()
    );
}
