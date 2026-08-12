#![cfg(feature = "msgpack")]

use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use xidl_jsonrpc::{Client, Error, Handler, Server};

struct EchoHandler;

#[async_trait::async_trait]
impl Handler for EchoHandler {
    async fn handle(&self, method: &str, params: Value) -> Result<Value, Error> {
        if method == "echo" {
            Ok(params)
        } else {
            Err(Error::method_not_found(method))
        }
    }
}

async fn read_frame<R>(reader: &mut R) -> std::io::Result<Option<Vec<u8>>>
where
    R: AsyncReadExt + Unpin,
{
    let mut len = [0_u8; 4];
    let mut first = [0_u8; 1];
    match reader.read(&mut first).await? {
        0 => return Ok(None),
        _ => len[0] = first[0],
    }
    reader.read_exact(&mut len[1..]).await?;
    let payload_len = u32::from_be_bytes(len) as usize;
    let mut payload = vec![0_u8; payload_len];
    reader.read_exact(&mut payload).await?;
    Ok(Some(payload))
}

async fn write_frame<W>(writer: &mut W, value: &Value) -> std::io::Result<()>
where
    W: AsyncWriteExt + Unpin,
{
    let payload = rmp_serde::to_vec(value).map_err(std::io::Error::other)?;
    writer
        .write_all(&(payload.len() as u32).to_be_bytes())
        .await?;
    writer.write_all(&payload).await?;
    writer.flush().await?;
    Ok(())
}

#[tokio::test]
async fn client_new_msgpack_round_trips_with_raw_server() {
    let (mut client_stream, mut server_stream) = tokio::io::duplex(512);
    let server = tokio::spawn(async move {
        let Some(payload) = read_frame(&mut server_stream).await.unwrap() else {
            panic!("expected request frame");
        };
        let request: Value = rmp_serde::from_slice(&payload).unwrap();
        assert_eq!(request["method"], "echo");
        let response = json!({
            "jsonrpc": "2.0",
            "id": request["id"],
            "result": request["params"],
        });
        write_frame(&mut server_stream, &response).await.unwrap();
        server_stream.shutdown().await.unwrap();
    });

    let mut client = Client::new_msgpack(&mut client_stream);
    let result: Value = client.call("echo", json!({"ok": true})).await.unwrap();
    assert_eq!(result, json!({"ok": true}));
    server.await.unwrap();
}

#[tokio::test]
async fn server_with_msgpack_serves_msgpack_client() {
    let (client_stream, server_stream) = tokio::io::duplex(512);
    let server = Server::builder()
        .with_msgpack()
        .with_stream(server_stream)
        .with_service(EchoHandler)
        .serve();
    let server_task = tokio::spawn(server);

    let mut client = Client::new_msgpack(client_stream);
    let result: Value = client.call("echo", json!([1, 2, 3])).await.unwrap();
    assert_eq!(result, json!([1, 2, 3]));

    server_task.await.unwrap().unwrap();
}

#[tokio::test]
async fn server_reports_parse_error_for_bad_msgpack_and_continues() {
    let (mut client_stream, server_stream) = tokio::io::duplex(512);
    let server = Server::builder()
        .with_msgpack()
        .with_stream(server_stream)
        .with_service(EchoHandler)
        .serve();
    let server_task = tokio::spawn(server);

    // Corrupt frame: a payload holding the reserved msgpack marker 0xc1,
    // which rmp_serde rejects as a decode error.
    client_stream.write_all(&[0, 0, 0, 1, 0xc1]).await.unwrap();
    // Valid request after the corrupt frame.
    write_frame(
        &mut client_stream,
        &json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "echo",
            "params": "ok",
        }),
    )
    .await
    .unwrap();

    let first = read_frame(&mut client_stream).await.unwrap().unwrap();
    let first: Value = rmp_serde::from_slice(&first).unwrap();
    assert_eq!(first["id"], Value::Null);
    assert_eq!(first["error"]["code"], -32700);

    let second = read_frame(&mut client_stream).await.unwrap().unwrap();
    let second: Value = rmp_serde::from_slice(&second).unwrap();
    assert_eq!(second["id"], 7);
    assert_eq!(second["result"], "ok");

    server_task.await.unwrap().unwrap();
}

#[tokio::test]
async fn server_reports_parse_error_for_oversized_msgpack_and_continues() {
    let (mut client_stream, server_stream) = tokio::io::duplex(512);
    let server = Server::builder()
        .with_msgpack()
        .with_stream(server_stream)
        .with_service(EchoHandler)
        .serve();
    let server_task = tokio::spawn(server);

    // Feed an oversized frame: a 4MiB+1 length prefix followed by the full
    // declared payload, so the server can drain it and realign to the next
    // frame boundary instead of closing the connection.
    let payload_len = 4 * 1024 * 1024 + 1;
    client_stream
        .write_all(&(payload_len as u32).to_be_bytes())
        .await
        .unwrap();
    let chunk = vec![0x00_u8; 4096];
    let mut remaining = payload_len;
    while remaining > 0 {
        let n = remaining.min(chunk.len());
        client_stream.write_all(&chunk[..n]).await.unwrap();
        remaining -= n;
    }
    // A valid request after the oversized frame.
    write_frame(
        &mut client_stream,
        &json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "echo",
            "params": "ok",
        }),
    )
    .await
    .unwrap();

    // The oversized frame yields a -32700 parse error with no id...
    let first = read_frame(&mut client_stream).await.unwrap().unwrap();
    let first: Value = rmp_serde::from_slice(&first).unwrap();
    assert_eq!(first["id"], Value::Null);
    assert_eq!(first["error"]["code"], -32700);
    assert!(
        first["error"]["message"]
            .as_str()
            .unwrap()
            .contains("frame exceeds maximum length")
    );

    // ...and the follow-up request is still served.
    let second = read_frame(&mut client_stream).await.unwrap().unwrap();
    let second: Value = rmp_serde::from_slice(&second).unwrap();
    assert_eq!(second["id"], 7);
    assert_eq!(second["result"], "ok");

    server_task.await.unwrap().unwrap();
}
