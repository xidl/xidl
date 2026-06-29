use crate::Error;
use crate::stream::{
    ClientStreamWriter, Reader, ReaderWriter, boxed, open_bidi_client, open_server_stream_client,
    polling,
};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

#[tokio::test]
async fn reader_and_boxed_stream_work() {
    let mut reader = Reader::new(boxed(async_stream::try_stream! {
        yield 1_u32;
        yield 2_u32;
    }));

    assert_eq!(reader.read().await.unwrap().unwrap(), 1);
    let mut inner = reader.into_inner();
    assert_eq!(
        futures_util::StreamExt::next(&mut inner)
            .await
            .unwrap()
            .unwrap(),
        2
    );
}

#[tokio::test]
async fn polling_repeats_until_dropped() {
    let mut n = 0_u32;
    let mut stream = polling(
        move || {
            n += 1;
            async move { Ok::<_, Error>(n) }
        },
        std::time::Duration::from_millis(1),
    );

    assert_eq!(
        futures_util::StreamExt::next(&mut stream)
            .await
            .unwrap()
            .unwrap(),
        1
    );
    assert_eq!(
        futures_util::StreamExt::next(&mut stream)
            .await
            .unwrap()
            .unwrap(),
        2
    );
}

#[tokio::test]
async fn client_stream_writer_handles_close_cancel_and_failures() {
    let (tx, mut rx) = mpsc::channel(2);
    let response = tokio::spawn(async move {
        while rx.recv().await.is_some() {}
        Ok::<_, Error>("done")
    });
    let mut writer: ClientStreamWriter<u32, &str> = ClientStreamWriter::new(tx, response);
    writer.write(1_u32).await.unwrap();
    assert_eq!(writer.close().await.unwrap(), "done");

    let (tx, rx) = mpsc::channel::<Result<u32, Error>>(1);
    drop(rx);
    let response = tokio::spawn(async { Ok::<_, Error>(()) });
    let mut writer: ClientStreamWriter<u32, ()> = ClientStreamWriter::new(tx, response);
    assert!(matches!(
        writer.write(1_u32).await,
        Err(Error::Protocol("stream writer is closed"))
    ));

    let (tx, rx) = mpsc::channel::<Result<u32, Error>>(1);
    drop(rx);
    let response = tokio::spawn(async { Ok::<_, Error>("done") });
    let writer = ClientStreamWriter::new(tx, response);
    writer.cancel().await.unwrap();
}

#[tokio::test]
async fn client_stream_writer_reports_double_close_and_join_failures() {
    let (tx, mut rx) = mpsc::channel(1);
    let response = tokio::spawn(async move {
        let _ = rx.recv().await;
        Ok::<_, Error>(())
    });
    let mut writer: ClientStreamWriter<u32, ()> = ClientStreamWriter::new(tx, response);
    let _ = writer.response.take();
    let err = writer.close().await.unwrap_err();
    assert!(matches!(
        err,
        Error::Protocol("stream writer is already closed")
    ));

    let (tx, _rx) = mpsc::channel::<Result<u32, Error>>(1);
    let response = tokio::spawn(async move {
        panic!("boom");
        #[allow(unreachable_code)]
        Ok::<_, Error>(())
    });
    let writer: ClientStreamWriter<u32, ()> = ClientStreamWriter::new(tx, response);
    let err = writer.close().await.unwrap_err();
    assert!(matches!(
        err,
        Error::Protocol("stream response task failed")
    ));
}

#[tokio::test]
async fn duplex_stream_round_trips_and_splits() {
    let (client, mut server) = tokio::io::duplex(512);
    let mut bidi = open_bidi_client(client, "stream").await.unwrap();

    let mut request = [0_u8; 128];
    let n = server.read(&mut request).await.unwrap();
    assert_eq!(
        std::str::from_utf8(&request[..n]).unwrap(),
        "{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"stream\",\"params\":null}\n"
    );

    server.write_all(b"{\"reply\":1}\n").await.unwrap();
    server.shutdown().await.unwrap();
    assert_eq!(bidi.read().await.unwrap().unwrap(), json!({"reply": 1}));
    bidi.cancel().await.unwrap();

    let (tx, rx) = mpsc::channel(1);
    let response = tokio::spawn(async { Ok::<_, Error>(()) });
    let duplex: ReaderWriter<serde_json::Value, serde_json::Value> = ReaderWriter::new(
        ClientStreamWriter::new(tx, response),
        Reader::new(boxed(async_stream::try_stream! {
            yield json!(1);
        })),
    );
    let (_writer, mut reader) = duplex.into_parts();
    assert_eq!(reader.read().await.unwrap().unwrap(), json!(1));
    drop(rx);
}

#[tokio::test]
async fn open_server_stream_client_reads_streamed_values() {
    let (client, mut server) = tokio::io::duplex(512);
    let mut reader = open_server_stream_client(client, "events", json!({"start": true}))
        .await
        .unwrap();

    let mut request = [0_u8; 128];
    let n = server.read(&mut request).await.unwrap();
    assert_eq!(
        std::str::from_utf8(&request[..n]).unwrap(),
        "{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"events\",\"params\":{\"start\":true}}\n"
    );

    server.write_all(b"{\"tick\":1}\n").await.unwrap();
    server.shutdown().await.unwrap();
    assert_eq!(reader.read().await.unwrap().unwrap(), json!({"tick": 1}));
    assert!(reader.read().await.is_none());
}

#[tokio::test]
async fn json_value_reader_reads_until_eof() {
    let (mut writer, reader) = tokio::io::duplex(256);
    let mut reader = super::json_value_reader(reader);

    writer.write_all(b"{\"a\":1}\n{\"b\":2}\n").await.unwrap();
    writer.shutdown().await.unwrap();

    assert_eq!(reader.read().await.unwrap().unwrap(), json!({"a": 1}));
    assert_eq!(reader.read().await.unwrap().unwrap(), json!({"b": 2}));
    assert!(reader.read().await.is_none());
}
