use super::*;
use crate::Error;
#[cfg(feature = "server")]
use axum::body::{self, Body};
use futures_util::{StreamExt, stream};
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::AsyncRead;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct Payload {
    value: u32,
}

#[tokio::test]
async fn reader_reads_items_from_boxed_stream() {
    let stream = Box::pin(stream::iter(vec![Ok::<_, StreamError<Error>>(Payload {
        value: 1,
    })]));
    let mut reader = Reader::new(stream);
    assert_eq!(reader.read().await.unwrap().unwrap(), Payload { value: 1 });
    assert!(reader.read().await.is_none());
    assert_eq!(reader.outcome(), Some(StreamOutcome::Completed));
    reader.cancel();
    assert_eq!(reader.outcome(), Some(StreamOutcome::Completed));
    let mut reader = Reader::new(Box::pin(stream::pending::<
        std::result::Result<Payload, StreamError<Error>>,
    >()));
    reader.cancel();
    assert!(reader.read().await.is_none());
    assert_eq!(reader.outcome(), Some(StreamOutcome::Cancelled));
}

#[cfg(feature = "server")]
#[tokio::test]
async fn bidi_server_stream_write_close_and_error_sender_follow_state() {
    let (in_tx, in_rx) = mpsc::channel(1);
    let (out_tx, mut out_rx) = mpsc::channel(1);
    let mut stream = BidiServerStream {
        inbound: in_rx,
        outbound: Some(out_tx),
        task: None,
    };

    in_tx.send(Ok(Payload { value: 9 })).await.unwrap();
    drop(in_tx);
    assert_eq!(stream.read().await.unwrap().unwrap(), Payload { value: 9 });

    stream.write(Payload { value: 10 }).await.unwrap();
    assert_eq!(out_rx.recv().await.unwrap().unwrap(), Payload { value: 10 });
    assert!(stream.error_sender().is_some());

    stream.close();
    assert!(stream.error_sender().is_none());
    let err = stream.write(Payload { value: 11 }).await.unwrap_err();
    assert_eq!(err.code, 500);
}

#[tokio::test]
async fn bidi_client_stream_write_read_close_and_cancel_follow_state() {
    let (writer_tx, mut writer_rx) = mpsc::channel(1);
    let (reader_tx, reader_rx) = mpsc::channel(1);
    let write_task = tokio::spawn(async move {
        tokio::task::yield_now().await;
    });
    let read_task = tokio::spawn(async move {
        tokio::task::yield_now().await;
    });
    let mut stream = BidiClientStream {
        writer: Some(writer_tx),
        reader: reader_rx,
        write_task: Some(write_task),
        read_task: Some(read_task),
    };

    stream.write(Payload { value: 1 }).await.unwrap();
    assert_eq!(
        writer_rx.recv().await.unwrap().unwrap(),
        Payload { value: 1 }
    );

    reader_tx.send(Ok(Payload { value: 2 })).await.unwrap();
    assert_eq!(stream.read().await.unwrap().unwrap(), Payload { value: 2 });

    stream.close();
    let err = stream.write(Payload { value: 3 }).await.unwrap_err();
    assert_eq!(err.code, 500);

    stream.cancel();
}

#[tokio::test]
async fn client_stream_writer_close_and_cancel_handle_state() {
    let (tx, mut rx) = mpsc::channel(1);
    let writer = ClientStreamWriter::new(
        tx,
        tokio::spawn(async move { Ok::<_, Error>(Payload { value: 42 }) }),
    );
    let mut writer = writer;
    writer.write(Payload { value: 7 }).await.unwrap();
    assert_eq!(rx.recv().await.unwrap().unwrap(), Payload { value: 7 });
    assert_eq!(writer.close().await.unwrap(), Payload { value: 42 });

    let (tx, _rx) = mpsc::channel(1);
    let writer = ClientStreamWriter::<Payload, Payload>::new(
        tx,
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            Ok::<_, Error>(Payload { value: 0 })
        }),
    );
    writer.cancel().await.unwrap();
}

#[cfg(feature = "server")]
#[tokio::test]
async fn sse_response_stops_after_the_error_without_completing() {
    let response = sse_response(boxed_sse(stream::iter(vec![
        Ok(Payload { value: 5 }),
        Err(Error::new(409, "boom")),
    ])))
    .await;
    let bytes = body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(text.contains("event: next"));
    assert!(text.contains(r#"data: {"value":5}"#));
    assert!(text.contains("event: error"));
    assert!(text.contains(r#"data: {"status":409,"body":{"code":409,"msg":"boom"}}"#));
    assert!(!text.contains("event: complete"));
}

#[cfg(feature = "server")]
#[tokio::test]
async fn decode_ndjson_body_decodes_valid_and_invalid_lines() {
    let body = Body::from("{\"t\":\"next\",\"data\":{\"value\":1}}\nnot-json\n");
    let mut stream = decode_ndjson_body::<Payload>(body);

    assert_eq!(stream.next().await.unwrap().unwrap(), Payload { value: 1 });
    let err = stream.next().await.unwrap().unwrap_err();
    assert_eq!(err.code, 400);
}

#[test]
fn sse_decode_state_ignores_comments_and_decodes_terminal_payloads() {
    let mut state = SseDecodeState::default();
    for line in [": heartbeat", "", "event: next", "data: {\"value\":3}"] {
        assert!(matches!(
            state.push_line::<Payload, Error>(line).unwrap(),
            StreamAction::Continue
        ));
    }
    assert!(matches!(
        state.push_line::<Payload, Error>("").unwrap(),
        StreamAction::Item(Payload { value: 3 })
    ));
    for line in ["event: complete", "data: {}"] {
        state.push_line::<Payload, Error>(line).unwrap();
    }
    assert!(matches!(
        state.push_line::<Payload, Error>("").unwrap(),
        StreamAction::Done
    ));
    for line in [
        "event: error",
        "data: {\"status\":418,\"body\":{\"code\":418,\"msg\":\"teapot\"}}",
    ] {
        state.push_line::<Payload, Error>(line).unwrap();
    }
    match state.push_line::<Payload, Error>("").unwrap() {
        StreamAction::Failed(error) => assert_eq!(error.code, 418),
        _ => panic!("expected a terminal failure"),
    }
    state.push_line::<Payload, Error>("data: not-json").unwrap();
    assert!(state.push_line::<Payload, Error>("").is_err());
}

struct BrokenReader;

impl AsyncRead for BrokenReader {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Ready(Err(std::io::Error::other("broken reader")))
    }
}

#[tokio::test]
async fn decode_ndjson_reader_surfaces_io_errors() {
    let mut stream = decode_ndjson_reader::<Payload, _>(BrokenReader);
    let err = stream.next().await.unwrap().unwrap_err();
    assert_eq!(err.code, 400);
    assert!(stream.next().await.is_none());
}

#[test]
fn open_bidi_client_rejects_invalid_urls_before_connecting() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let err = match runtime.block_on(open_bidi_client::<Payload, Payload>("not a url")) {
        Err(err) => err,
        Ok(_) => panic!("expected invalid url error"),
    };
    assert_eq!(err.code, 500);
}

#[test]
fn open_bidi_client_with_headers_rejects_invalid_urls_before_connecting() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let err = match runtime.block_on(open_bidi_client_with_headers::<Payload, Payload>(
        "not a url",
        http::HeaderMap::new(),
    )) {
        Err(err) => err,
        Ok(_) => panic!("expected invalid url error"),
    };
    assert_eq!(err.code, 500);
}
