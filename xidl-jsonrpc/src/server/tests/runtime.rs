use crate::server::runtime::{Io, Server};
use crate::transport::Listener;
use crate::{Error, Handler};
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::Arc;

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
        #[cfg(not(tarpaulin_include))]
        tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};

            let mut client = client;
            let mut buf = [0_u8; 128];
            let _ = client.read(&mut buf).await.unwrap();
            client
                .write_all(br#"{"jsonrpc":"2.0","id":1,"result":null}"#)
                .await
                .unwrap();
            client.write_all(b"\n").await.unwrap();
            client.shutdown().await.unwrap();
        });
        #[cfg(tarpaulin_include)]
        drop(client);

        Ok((Box::new(server), SocketAddr::from(([127, 0, 0, 1], 0))))
    }
}

#[tokio::test]
async fn io_constructor_sets_fields() {
    let (reader, writer) = tokio::io::duplex(64);
    let io = Io::new(reader, writer);
    let _ = io.reader;
    let _ = io.writer;
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
    let (reader, writer) = tokio::io::duplex(64);
    let server = Server::builder()
        .with_service(StubHandler)
        .with_io(Io::new(reader, writer))
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
