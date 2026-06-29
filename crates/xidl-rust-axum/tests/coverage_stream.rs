#![cfg(not(tarpaulin_include))]

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::body::{self, Body};
use axum::extract::ws::{Message, WebSocketUpgrade};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::{SinkExt, StreamExt, stream};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;
use xidl_rust_axum::stream::{
    boxed_ndjson, boxed_sse, decode_ndjson_body, encode_ndjson_body, open_bidi_client,
    open_bidi_client_with_headers, open_bidi_server, open_sse, sse_response,
};
use xidl_rust_axum::{Error, ErrorBody};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
struct Payload {
    value: u32,
}

async fn spawn_router(router: Router) -> (SocketAddr, JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });
    (addr, handle)
}

async fn stop_server(handle: JoinHandle<()>) {
    handle.abort();
    let _ = handle.await;
}

#[tokio::test]
async fn bidi_client_and_server_round_trip_and_transport_errors() {
    let app = Router::new().route(
        "/ws",
        get(|ws: WebSocketUpgrade| async move {
            ws.on_upgrade(|socket| async move {
                let mut stream = open_bidi_server::<Payload, Payload>(socket);
                assert_eq!(stream.read().await.unwrap().unwrap(), Payload { value: 1 });
                stream.write(Payload { value: 2 }).await.unwrap();
                stream
                    .error_sender()
                    .unwrap()
                    .send(Err(Error::new(409, "boom")))
                    .await
                    .unwrap();
            })
        }),
    );
    let (addr, handle) = spawn_router(app).await;

    let mut client = open_bidi_client::<Payload, Payload>(&format!("ws://{addr}/ws"))
        .await
        .unwrap();
    client.write(Payload { value: 1 }).await.unwrap();
    assert_eq!(client.read().await.unwrap().unwrap(), Payload { value: 2 });
    let err = client.read().await.unwrap().unwrap_err();
    assert_eq!(err.code, 409);

    stop_server(handle).await;
}

#[tokio::test]
async fn bidi_server_reports_invalid_json_payloads() {
    let (tx, rx) = oneshot::channel();
    let tx = Arc::new(Mutex::new(Some(tx)));
    let app = Router::new().route(
        "/ws",
        get(move |ws: WebSocketUpgrade| {
            let tx = Arc::clone(&tx);
            async move {
                ws.on_upgrade(move |socket| async move {
                    let mut stream = open_bidi_server::<Payload, Payload>(socket);
                    let result = stream.read().await.unwrap();
                    tx.lock().unwrap().take().unwrap().send(result).unwrap();
                })
            }
        }),
    );
    let (addr, handle) = spawn_router(app).await;

    let (mut socket, _) = connect_async(format!("ws://{addr}/ws")).await.unwrap();
    socket
        .send(TungsteniteMessage::Text("not-json".into()))
        .await
        .unwrap();
    let err = rx.await.unwrap().unwrap_err();
    assert_eq!(err.code, 400);

    stop_server(handle).await;
}

#[tokio::test]
async fn bidi_client_reports_invalid_server_payloads_and_custom_headers() {
    let app = Router::new()
        .route(
            "/invalid",
            get(|ws: WebSocketUpgrade| async move {
                ws.on_upgrade(|mut socket| async move {
                    socket.send(Message::Text("not-json".into())).await.unwrap();
                })
            }),
        )
        .route(
            "/headers",
            get(|headers: HeaderMap, ws: WebSocketUpgrade| async move {
                assert_eq!(headers.get("x-test").unwrap(), "header-value");
                ws.on_upgrade(|socket| async move {
                    let mut stream = open_bidi_server::<Payload, Payload>(socket);
                    let inbound = stream.read().await.unwrap().unwrap();
                    stream.write(inbound).await.unwrap();
                })
            }),
        );
    let (addr, handle) = spawn_router(app).await;

    let mut client = open_bidi_client::<Payload, Payload>(&format!("ws://{addr}/invalid"))
        .await
        .unwrap();
    let err = client.read().await.unwrap().unwrap_err();
    assert_eq!(err.code, 400);

    let mut headers = HeaderMap::new();
    headers.insert("x-test", "header-value".parse().unwrap());
    let mut client =
        open_bidi_client_with_headers::<Payload, Payload>(&format!("ws://{addr}/headers"), headers)
            .await
            .unwrap();
    client.write(Payload { value: 7 }).await.unwrap();
    assert_eq!(client.read().await.unwrap().unwrap(), Payload { value: 7 });

    stop_server(handle).await;
}

#[tokio::test]
async fn open_sse_covers_success_and_error_responses() {
    let app = Router::new()
        .route(
            "/ok",
            get(|| async {
                sse_response(boxed_sse(stream::iter(vec![
                    Ok(Payload { value: 1 }),
                    Err(Error::new(410, "gone")),
                ])))
            }),
        )
        .route(
            "/err",
            get(|| async {
                (
                    StatusCode::NOT_FOUND,
                    Json(ErrorBody {
                        code: 404,
                        msg: "missing".into(),
                    }),
                )
            }),
        )
        .route(
            "/complete",
            get(|| async { sse_response(boxed_sse(stream::iter(vec![Ok(Payload { value: 9 })]))) }),
        )
        .route(
            "/eof",
            get(|| async {
                (
                    [(axum::http::header::CONTENT_TYPE, "text/event-stream")],
                    "event: next\ndata: {\"value\":4}\n",
                )
            }),
        );
    let (addr, handle) = spawn_router(app).await;
    let http = reqwest::Client::new();

    let req = http.get(format!("http://{addr}/ok")).build().unwrap();
    let mut reader = open_sse::<Payload>(&http, req).await.unwrap();
    assert_eq!(reader.read().await.unwrap().unwrap(), Payload { value: 1 });
    let err = reader.read().await.unwrap().unwrap_err();
    assert_eq!(err.code, 410);

    let req = http.get(format!("http://{addr}/err")).build().unwrap();
    let err = match open_sse::<Payload>(&http, req).await {
        Err(err) => err,
        Ok(_) => panic!("expected non-success response"),
    };
    assert_eq!(err.code, 404);

    let req = http.get(format!("http://{addr}/complete")).build().unwrap();
    let mut reader = open_sse::<Payload>(&http, req).await.unwrap();
    assert_eq!(reader.read().await.unwrap().unwrap(), Payload { value: 9 });
    assert!(reader.read().await.is_none());

    let req = http.get(format!("http://{addr}/eof")).build().unwrap();
    let mut reader = open_sse::<Payload>(&http, req).await.unwrap();
    assert!(reader.read().await.is_none());

    stop_server(handle).await;
}

#[tokio::test]
async fn ndjson_helpers_cover_success_streaming_and_stream_errors() {
    let app = Router::new().route(
        "/echo",
        post(|body: Body| async move {
            let bytes = body::to_bytes(body, usize::MAX).await.unwrap();
            String::from_utf8(bytes.to_vec()).unwrap()
        }),
    );
    let (addr, handle) = spawn_router(app).await;
    let http = reqwest::Client::new();

    let body = encode_ndjson_body(boxed_ndjson(stream::iter(vec![
        Ok(Payload { value: 1 }),
        Ok(Payload { value: 2 }),
    ])));
    let text = http
        .post(format!("http://{addr}/echo"))
        .body(body)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert_eq!(text, "{\"value\":1}\n{\"value\":2}\n");

    let mut stream = decode_ndjson_body::<Payload>(Body::from("{\"value\":3}\nnot-json\n"));
    assert_eq!(stream.next().await.unwrap().unwrap(), Payload { value: 3 });
    let err = stream.next().await.unwrap().unwrap_err();
    assert_eq!(err.code, 400);

    let body = encode_ndjson_body::<Payload>(boxed_ndjson(stream::iter(vec![Err(Error::new(
        500, "fail",
    ))])));
    let result = http
        .post(format!("http://{addr}/echo"))
        .body(body)
        .send()
        .await;
    match result {
        Ok(response) => {
            let _ = response.text().await.unwrap();
        }
        Err(err) => {
            assert!(err.is_body() || err.is_request());
        }
    }

    stop_server(handle).await;
}

#[tokio::test]
async fn decode_ndjson_body_reports_codec_errors_for_long_lines() {
    let long_line = format!("{}\n", "x".repeat(9_000));
    let mut stream = decode_ndjson_body::<Payload>(Body::from(long_line));
    let err = stream.next().await.unwrap().unwrap_err();
    assert_eq!(err.code, 400);
}
