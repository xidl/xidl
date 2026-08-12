//! End-to-end JSON-RPC protocol integration tests.
//!
//! These tests exercise the public `Client`, `Server`, `Handler`, and `stream`
//! APIs together over a real inproc transport. They complement the
//! transport-focused `serve_on` tests and the codec-focused `msgpack` tests.

#![cfg(feature = "tokio")]

use serde_json::{Value, json};
use std::time::Duration;
use xidl_jsonrpc::stream::ReaderWriter;
use xidl_jsonrpc::{Client, Error, Handler, Server};

fn random_endpoint(prefix: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before epoch")
        .as_nanos();
    format!("{prefix}-{nanos}")
}

async fn connect_with_retry(
    endpoint: &str,
) -> std::io::Result<Box<dyn xidl_jsonrpc::transport::Stream + Unpin + Send + 'static>> {
    let mut last_err = None;
    for _ in 0..50 {
        match xidl_jsonrpc::connect(endpoint).await {
            Ok(stream) => return Ok(stream),
            Err(err) => {
                last_err = Some(err);
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
    }
    Err(last_err.unwrap_or_else(|| {
        std::io::Error::other(format!("failed to connect endpoint: {endpoint}"))
    }))
}

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

struct AlphaHandler;

#[async_trait::async_trait]
impl Handler for AlphaHandler {
    async fn handle(&self, method: &str, params: Value) -> Result<Value, Error> {
        match method {
            "alpha" => Ok(json!({ "alpha": params })),
            _ => Err(Error::method_not_found(method)),
        }
    }
}

struct BetaHandler;

#[async_trait::async_trait]
impl Handler for BetaHandler {
    async fn handle(&self, method: &str, params: Value) -> Result<Value, Error> {
        match method {
            "beta" => Ok(json!({ "beta": params })),
            _ => Err(Error::method_not_found(method)),
        }
    }
}

struct FailingHandler;

#[async_trait::async_trait]
impl Handler for FailingHandler {
    async fn handle(&self, method: &str, _params: Value) -> Result<Value, Error> {
        match method {
            "fail" => Err(Error::invalid_params("bad params")),
            _ => Err(Error::method_not_found(method)),
        }
    }
}

struct EchoBidiHandler;

#[async_trait::async_trait]
impl Handler for EchoBidiHandler {
    async fn handle(&self, method: &str, _params: Value) -> Result<Value, Error> {
        Err(Error::method_not_found(method))
    }

    fn accepts_bidi(&self, method: &str) -> bool {
        method == "bidi"
    }

    async fn handle_bidi(
        &self,
        method: &str,
        _params: Value,
        mut stream: ReaderWriter<Value, Value>,
    ) -> Result<(), Error> {
        if method != "bidi" {
            return Err(Error::method_not_found(method));
        }
        while let Some(item) = stream.read().await {
            let value = item?;
            stream.write(value).await?;
        }
        stream.close().await
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn one_connection_serves_sequential_calls() {
    let uri = format!("inproc://{}", random_endpoint("sequential"));
    let serve_uri = uri.clone();
    let server = tokio::spawn(async move {
        Server::builder()
            .with_service(EchoHandler)
            .serve_on(&serve_uri)
            .await
    });

    let stream = connect_with_retry(&uri).await.expect("connect inproc");
    let mut client = Client::new(stream);

    let first: Value = client
        .call("echo", json!({ "n": 1 }))
        .await
        .expect("first call");
    assert_eq!(first, json!({ "n": 1 }));

    let second: Value = client
        .call("echo", json!({ "n": 2 }))
        .await
        .expect("second call");
    assert_eq!(second, json!({ "n": 2 }));

    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn multiple_services_dispatch_by_method() {
    let uri = format!("inproc://{}", random_endpoint("dispatch"));
    let serve_uri = uri.clone();
    let server = tokio::spawn(async move {
        Server::builder()
            .with_service(AlphaHandler)
            .with_service(BetaHandler)
            .serve_on(&serve_uri)
            .await
    });

    let stream = connect_with_retry(&uri).await.expect("connect inproc");
    let mut client = Client::new(stream);

    let alpha: Value = client.call("alpha", json!(1)).await.expect("alpha call");
    assert_eq!(alpha, json!({ "alpha": 1 }));

    let beta: Value = client.call("beta", json!(2)).await.expect("beta call");
    assert_eq!(beta, json!({ "beta": 2 }));

    let err = client
        .call::<_, Value>("nope", json!(null))
        .await
        .expect_err("unknown method should fail");
    assert!(matches!(
        &err,
        Error::Rpc { message, .. } if message.contains("method not found")
    ));

    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn handler_errors_reach_the_client_as_rpc_errors() {
    let uri = format!("inproc://{}", random_endpoint("errors"));
    let serve_uri = uri.clone();
    let server = tokio::spawn(async move {
        Server::builder()
            .with_service(FailingHandler)
            .serve_on(&serve_uri)
            .await
    });

    let stream = connect_with_retry(&uri).await.expect("connect inproc");
    let mut client = Client::new(stream);

    let err = client
        .call::<_, Value>("fail", json!({}))
        .await
        .expect_err("handler error should fail the call");
    assert!(matches!(
        &err,
        Error::Rpc { message, data, .. } if message == "bad params" && data.is_none()
    ));

    let err = client
        .call::<_, Value>("missing", json!({}))
        .await
        .expect_err("unknown method should fail the call");
    assert!(matches!(
        &err,
        Error::Rpc { message, .. } if message == "method not found: missing"
    ));

    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bidi_stream_round_trips_through_server() {
    let uri = format!("inproc://{}", random_endpoint("bidi"));
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

    tokio::time::timeout(Duration::from_secs(10), async {
        // open_bidi_client already consumed the server's handshake
        // acknowledgement (result null, id 1), so the first value read is
        // the first echoed stream item.
        bidi.write(json!({ "n": 1 })).await?;
        let first = bidi.read().await.expect("first echo expected")?;
        assert_eq!(first, json!({ "n": 1 }));

        bidi.write(json!({ "n": 2 })).await?;
        let second = bidi.read().await.expect("second echo expected")?;
        assert_eq!(second, json!({ "n": 2 }));

        bidi.close().await?;
        Ok::<(), Error>(())
    })
    .await
    .expect("bidi exchange timed out")
    .expect("bidi exchange failed");

    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn server_serves_concurrent_clients() {
    let uri = format!("inproc://{}", random_endpoint("concurrent"));
    let serve_uri = uri.clone();
    let server = tokio::spawn(async move {
        Server::builder()
            .with_service(EchoHandler)
            .serve_on(&serve_uri)
            .await
    });

    let stream_a = connect_with_retry(&uri).await.expect("connect client a");
    let stream_b = connect_with_retry(&uri).await.expect("connect client b");

    let client_a = tokio::spawn(async move {
        let mut client = Client::new(stream_a);
        client
            .call::<_, Value>("echo", json!({ "client": "a" }))
            .await
    });
    let client_b = tokio::spawn(async move {
        let mut client = Client::new(stream_b);
        client
            .call::<_, Value>("echo", json!({ "client": "b" }))
            .await
    });

    assert_eq!(
        client_a.await.expect("join client a").expect("call a"),
        json!({ "client": "a" })
    );
    assert_eq!(
        client_b.await.expect("join client b").expect("call b"),
        json!({ "client": "b" })
    );

    server.abort();
}
