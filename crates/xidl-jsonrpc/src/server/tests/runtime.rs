use crate::server::runtime::Server;
use crate::transport::Listener;
use crate::{Error, Handler};
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::Arc;
#[cfg(all(feature = "transport-ipc", unix))]
use std::time::{SystemTime, UNIX_EPOCH};

struct StubHandler;

#[async_trait::async_trait]
impl Handler for StubHandler {
    async fn handle(&self, _method: &str, _params: Value) -> Result<Value, Error> {
        Ok(Value::Null)
    }
}

struct BrokenPipeListener;

#[async_trait::async_trait]
impl Listener for BrokenPipeListener {
    async fn accept(
        &self,
    ) -> std::io::Result<(
        Box<dyn crate::transport::Stream + Unpin + Send + 'static>,
        SocketAddr,
    )> {
        Err(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "listener closed",
        ))
    }

    fn endpoint(&self) -> Option<String> {
        Some("inproc://broken".to_string())
    }
}

struct SingleAcceptListener {
    accepted: tokio::sync::Mutex<bool>,
    done: Arc<tokio::sync::Notify>,
}

#[cfg(all(feature = "transport-ipc", unix))]
fn unique_ipc_endpoint(label: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock drift")
        .as_nanos();
    format!("ipc:///tmp/xjr-{label}-{}-{nanos}.sock", std::process::id())
}

#[async_trait::async_trait]
impl Listener for SingleAcceptListener {
    async fn accept(
        &self,
    ) -> std::io::Result<(
        Box<dyn crate::transport::Stream + Unpin + Send + 'static>,
        SocketAddr,
    )> {
        let mut accepted = self.accepted.lock().await;
        if *accepted {
            return Err(std::io::Error::other("accept failed"));
        }
        *accepted = true;

        let (client, server) = tokio::io::duplex(128);
        let done = self.done.clone();
        tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};

            let mut client = client;
            client
                .write_all(br#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#)
                .await
                .unwrap();
            client.write_all(b"\n").await.unwrap();
            // The session keeps its read half open (it loops until EOF), so the
            // response arrives line-delimited; read until the trailing newline.
            let mut buf = Vec::new();
            let mut chunk = [0_u8; 64];
            loop {
                let n = client.read(&mut chunk).await.unwrap();
                assert!(n > 0, "unexpected EOF while awaiting response");
                buf.extend_from_slice(&chunk[..n]);
                if buf.ends_with(b"\n") {
                    break;
                }
            }
            assert_eq!(buf, b"{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":null}\n");
            done.notify_one();
        });

        Ok((Box::new(server), SocketAddr::from(([127, 0, 0, 1], 0))))
    }
}

struct ReadErrorStream {
    attempted: Arc<tokio::sync::Notify>,
}

impl tokio::io::AsyncRead for ReadErrorStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        _buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        self.attempted.notify_one();
        std::task::Poll::Ready(Err(std::io::Error::other("read failed")))
    }
}

impl tokio::io::AsyncWrite for ReadErrorStream {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        _buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        std::task::Poll::Ready(Err(std::io::Error::other("write failed")))
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }
}

struct ReadErrorListener {
    attempted: Arc<tokio::sync::Notify>,
    accepted: tokio::sync::Mutex<bool>,
}

#[async_trait::async_trait]
impl Listener for ReadErrorListener {
    async fn accept(
        &self,
    ) -> std::io::Result<(
        Box<dyn crate::transport::Stream + Unpin + Send + 'static>,
        SocketAddr,
    )> {
        let mut accepted = self.accepted.lock().await;
        if *accepted {
            // Wait until the first session has attempted its failing read so
            // its error is reported before this accept fails.
            self.attempted.notified().await;
            return Err(std::io::Error::other("accept failed"));
        }
        *accepted = true;

        Ok((
            Box::new(ReadErrorStream {
                attempted: self.attempted.clone(),
            }),
            SocketAddr::from(([127, 0, 0, 1], 0)),
        ))
    }
}

#[tokio::test]
async fn builder_rejects_invalid_binding_configurations() {
    let err = match Server::builder().build().await {
        Ok(_) => panic!("expected missing listener"),
        Err(err) => err,
    };
    assert!(matches!(err, Error::Protocol("missing listener")));

    let err = match Server::builder()
        .with_listener(BrokenPipeListener)
        .with_endpoint("inproc://dup")
        .build()
        .await
    {
        Ok(_) => panic!("expected listener conflict"),
        Err(err) => err,
    };
    assert!(matches!(err, Error::Protocol("listener already set")));

    #[cfg(not(all(feature = "transport-ipc", unix)))]
    {
        let err = match Server::builder()
            .with_service(StubHandler)
            .serve_on("ipc://unsupported")
            .await
        {
            Ok(_) => panic!("expected unsupported transport"),
            Err(err) => err,
        };
        assert_eq!(
            err.to_string(),
            "io error: ipc transport requires `transport-ipc` feature"
        );
    }
}

#[tokio::test]
async fn builder_resolves_listener_endpoint_and_service() {
    let server = Server::builder()
        .with_listener(BrokenPipeListener)
        .with_service(StubHandler)
        .build()
        .await
        .unwrap();

    assert_eq!(server.endpoint(), Some("inproc://broken"));
    server.serve().await.unwrap();
}

#[tokio::test]
async fn builder_supports_io_builders_and_endpoint_shortcuts() {
    let (stream, _peer) = tokio::io::duplex(64);
    let server = Server::builder()
        .with_service(StubHandler)
        .with_stream(stream)
        .build()
        .await
        .unwrap();
    assert_eq!(server.endpoint(), None);

    let server = Server::builder()
        .with_service(StubHandler)
        .build_on("inproc://runtime-shortcut")
        .await
        .unwrap();
    assert_eq!(server.endpoint(), Some("inproc://runtime-shortcut"));

    #[cfg(all(feature = "transport-ipc", unix))]
    {
        let endpoint = unique_ipc_endpoint("runtime-shortcut");
        let server = Server::builder()
            .with_service(StubHandler)
            .build_on(&endpoint)
            .await
            .unwrap();
        assert_eq!(server.endpoint(), Some(endpoint.as_str()));
    }

    #[cfg(not(all(feature = "transport-ipc", unix)))]
    {
        let err = match Server::builder()
            .with_service(StubHandler)
            .build_on("ipc://unsupported")
            .await
        {
            Ok(_) => panic!("expected unsupported transport"),
            Err(err) => err,
        };
        assert_eq!(
            err.to_string(),
            "io error: ipc transport requires `transport-ipc` feature"
        );
    }

    let result = Server::builder()
        .with_service(Arc::new(StubHandler))
        .with_listener(BrokenPipeListener)
        .serve()
        .await;
    assert!(result.is_ok());

    let err = Server::builder()
        .with_service(StubHandler)
        .with_listener(SingleAcceptListener {
            accepted: tokio::sync::Mutex::new(false),
            done: Arc::new(tokio::sync::Notify::new()),
        })
        .serve()
        .await
        .unwrap_err();
    assert_eq!(err.to_string(), "io error: accept failed");

    assert_eq!(
        StubHandler.handle("direct", Value::Null).await.unwrap(),
        Value::Null
    );
}

#[tokio::test]
async fn builder_accepts_max_in_flight_values() {
    for value in [0, 1, 7] {
        let server = Server::builder()
            .with_listener(BrokenPipeListener)
            .with_max_in_flight(value)
            .build()
            .await
            .unwrap();
        server.serve().await.unwrap();
    }
}

#[tokio::test]
async fn serve_completes_request_round_trip_over_accepted_stream() {
    let done = Arc::new(tokio::sync::Notify::new());
    let server = Server::builder()
        .with_service(StubHandler)
        .with_listener(SingleAcceptListener {
            accepted: tokio::sync::Mutex::new(false),
            done: done.clone(),
        })
        .build()
        .await
        .unwrap();
    let serve = tokio::spawn(async move { server.serve().await });
    done.notified().await;
    let err = serve.await.unwrap().unwrap_err();
    assert_eq!(err.to_string(), "io error: accept failed");
}

#[tokio::test]
async fn serve_reports_session_read_failures() {
    let attempted = Arc::new(tokio::sync::Notify::new());
    let server = Server::builder()
        .with_service(StubHandler)
        .with_listener(ReadErrorListener {
            attempted,
            accepted: tokio::sync::Mutex::new(false),
        })
        .build()
        .await
        .unwrap();
    let err = server.serve().await.unwrap_err();
    assert_eq!(err.to_string(), "io error: accept failed");
}
