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

#[tokio::test]
async fn json_codec_rejects_oversized_frames_and_resyncs() {
    let (mut writer, reader) = tokio::io::duplex(64 * 1024);
    let oversized = format!("{}x\n", "x".repeat(super::MAX_FRAME_LEN + 1));
    let writer_task = tokio::spawn(async move {
        writer.write_all(oversized.as_bytes()).await.unwrap();
        writer.write_all(b"{\"ok\":true}\n").await.unwrap();
        writer.shutdown().await.unwrap();
    });

    let mut reader = BufReader::new(reader);
    let err = Codec::Json
        .read::<_, serde_json::Value>(&mut reader)
        .await
        .expect_err("oversized frame should be rejected");
    assert!(matches!(
        &err,
        crate::Error::FrameTooLarge {
            max,
            framing: "json"
        } if *max == super::MAX_FRAME_LEN
    ));

    let next = Codec::Json
        .read::<_, serde_json::Value>(&mut reader)
        .await
        .unwrap()
        .expect("line after oversized frame should parse");
    assert_eq!(next, json!({"ok": true}));
    writer_task.await.unwrap();
}

#[cfg(feature = "msgpack")]
#[tokio::test]
async fn msgpack_codec_rejects_oversized_frames_and_resyncs() {
    let (mut writer, reader) = tokio::io::duplex(64 * 1024);
    let payload_len = super::MAX_FRAME_LEN + 1;
    let writer_task = tokio::spawn(async move {
        writer
            .write_all(&(payload_len as u32).to_be_bytes())
            .await
            .unwrap();
        // Feed the declared oversized payload so the reader can realign.
        let chunk = vec![0x00_u8; 4096];
        let mut remaining = payload_len;
        while remaining > 0 {
            let n = remaining.min(chunk.len());
            writer.write_all(&chunk[..n]).await.unwrap();
            remaining -= n;
        }
        let small = rmp_serde::to_vec(&json!({"ok": true})).unwrap();
        writer
            .write_all(&(small.len() as u32).to_be_bytes())
            .await
            .unwrap();
        writer.write_all(&small).await.unwrap();
        writer.shutdown().await.unwrap();
    });

    let mut reader = BufReader::new(reader);
    let err = Codec::Msgpack
        .read::<_, serde_json::Value>(&mut reader)
        .await
        .expect_err("oversized msgpack frame should be rejected");
    assert!(matches!(
        &err,
        crate::Error::FrameTooLarge {
            max,
            framing: "msgpack"
        } if *max == super::MAX_FRAME_LEN
    ));

    let next = Codec::Msgpack
        .read::<_, serde_json::Value>(&mut reader)
        .await
        .unwrap()
        .expect("frame after oversized payload should parse");
    assert_eq!(next, json!({"ok": true}));
    writer_task.await.unwrap();
}

#[tokio::test]
async fn json_codec_parses_line_without_trailing_newline_on_eof() {
    let (mut writer, reader) = tokio::io::duplex(256);
    writer.write_all(br#"{"value":1}"#).await.unwrap();
    writer.shutdown().await.unwrap();

    let mut reader = BufReader::new(reader);
    let value = Codec::Json
        .read::<_, serde_json::Value>(&mut reader)
        .await
        .unwrap()
        .expect("a line without a trailing newline must still parse on EOF");
    assert_eq!(value, json!({"value": 1}));
}

#[tokio::test]
async fn json_codec_reports_frame_too_large_when_oversize_line_ends_at_eof() {
    let (mut writer, reader) = tokio::io::duplex(64 * 1024);
    let oversized = "x".repeat(super::MAX_FRAME_LEN + 1);
    let writer_task = tokio::spawn(async move {
        writer.write_all(oversized.as_bytes()).await.unwrap();
        writer.shutdown().await.unwrap();
    });

    let mut reader = BufReader::new(reader);
    let err = Codec::Json
        .read::<_, serde_json::Value>(&mut reader)
        .await
        .expect_err("an unterminated oversized line must be rejected");
    assert!(matches!(
        &err,
        crate::Error::FrameTooLarge {
            max,
            framing: "json"
        } if *max == super::MAX_FRAME_LEN
    ));
    writer_task.await.unwrap();
}

#[cfg(feature = "msgpack")]
#[tokio::test]
async fn msgpack_codec_reports_frame_too_large_when_payload_is_truncated() {
    let (mut writer, reader) = tokio::io::duplex(64 * 1024);
    let payload_len = super::MAX_FRAME_LEN + 1;
    writer
        .write_all(&(payload_len as u32).to_be_bytes())
        .await
        .unwrap();
    // Only a fragment of the declared payload arrives before EOF.
    writer.write_all(&vec![0x00_u8; 1024]).await.unwrap();
    writer.shutdown().await.unwrap();

    let mut reader = BufReader::new(reader);
    let err = Codec::Msgpack
        .read::<_, serde_json::Value>(&mut reader)
        .await
        .expect_err("an oversized truncated msgpack frame must be rejected");
    assert!(matches!(
        &err,
        crate::Error::FrameTooLarge {
            max,
            framing: "msgpack"
        } if *max == super::MAX_FRAME_LEN
    ));
}
