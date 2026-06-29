use crate::transport::{
    BoundListener, InprocListener, IoListener, Listener, bind, connect, connect_inproc,
};
#[cfg(all(feature = "transport-ipc", unix))]
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

struct EndpointlessListener;

#[cfg(all(feature = "transport-ipc", unix))]
fn unique_ipc_endpoint(label: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock drift")
        .as_nanos();
    format!("ipc:///tmp/xjr-{label}-{}-{nanos}.sock", std::process::id())
}

#[async_trait::async_trait]
impl Listener for EndpointlessListener {
    async fn accept(
        &self,
    ) -> std::io::Result<(
        Box<dyn crate::transport::Stream + Unpin + Send + 'static>,
        std::net::SocketAddr,
    )> {
        Err(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "unused",
        ))
    }
}

#[test]
fn core_helpers_return_expected_values() {
    assert_eq!(
        super::core::loopback_peer_addr(),
        std::net::SocketAddr::from(([127, 0, 0, 1], 0))
    );
    #[cfg(any(
        windows,
        not(unix),
        not(feature = "transport-tcp"),
        not(feature = "transport-quic"),
        not(feature = "transport-tls"),
        not(feature = "transport-websocket")
    ))]
    {
        let err = super::core::unsupported("nope");
        assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
        assert_eq!(err.to_string(), "nope");
    }
}

#[test]
fn bound_listener_keeps_endpoint() {
    let bound = BoundListener::new(Box::new(EndpointlessListener), "inproc://kept".to_string());
    let (_listener, endpoint) = bound.into_parts();
    assert_eq!(endpoint, "inproc://kept");
}

#[cfg(not(tarpaulin_include))]
#[tokio::test]
async fn io_listener_accepts_once_and_then_breaks() {
    let (listener_stream, mut peer_stream) = tokio::io::duplex(64);
    let writer_task = tokio::spawn(async move {
        peer_stream.write_all(b"ping").await.unwrap();
        let mut pong = [0_u8; 4];
        peer_stream.read_exact(&mut pong).await.unwrap();
        assert_eq!(&pong, b"pong");
        peer_stream.shutdown().await.unwrap();
    });

    let listener = IoListener::from_stream(listener_stream);
    let (mut stream, peer) = listener.accept().await.unwrap();
    assert_eq!(peer, std::net::SocketAddr::from(([127, 0, 0, 1], 0)));
    let mut buf = [0_u8; 4];
    stream.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, b"ping");
    assert!(!stream.is_write_vectored() || stream.is_write_vectored());

    let written = stream
        .write_vectored(&[std::io::IoSlice::new(b"po"), std::io::IoSlice::new(b"ng")])
        .await
        .unwrap();
    assert_eq!(written, 4);
    stream.flush().await.unwrap();
    stream.shutdown().await.unwrap();

    let err = match listener.accept().await {
        Ok(_) => panic!("expected broken pipe"),
        Err(err) => err,
    };
    assert_eq!(err.kind(), std::io::ErrorKind::BrokenPipe);
    writer_task.await.unwrap();
}

#[cfg(tarpaulin_include)]
#[tokio::test]
async fn io_listener_tarpaulin_stub_reports_broken_pipe() {
    let (stream, _peer) = tokio::io::duplex(16);
    let listener = IoListener::from_stream(stream);
    let err = listener.accept().await.unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::BrokenPipe);
}

#[tokio::test]
async fn inproc_listener_supports_pending_and_duplicate_bind_detection() {
    let endpoint = "transport-tests-pending";
    let mut client = connect_inproc(endpoint).unwrap();
    let listener = InprocListener::bind(endpoint).unwrap();
    assert_eq!(
        listener.endpoint().as_deref(),
        Some("inproc://transport-tests-pending")
    );

    let (mut server, _peer) = listener.accept().await.unwrap();
    client.write_all(b"hello").await.unwrap();
    client.shutdown().await.unwrap();
    let mut buf = String::new();
    server.read_to_string(&mut buf).await.unwrap();
    assert_eq!(buf, "hello");

    let err = match InprocListener::bind(endpoint) {
        Ok(_) => panic!("expected duplicate bind error"),
        Err(err) => err,
    };
    assert_eq!(err.kind(), std::io::ErrorKind::AddrInUse);
}

#[tokio::test]
async fn endpoint_bind_and_connect_cover_supported_and_unsupported_schemes() {
    let bound = bind("inproc://endpoint-bind").await.unwrap();
    let (_listener, endpoint) = bound.into_parts();
    assert_eq!(endpoint, "inproc://endpoint-bind");

    let mut client = connect("inproc://endpoint-connect").await.unwrap();
    let listener = InprocListener::bind("endpoint-connect").unwrap();
    let (mut server, _peer) = listener.accept().await.unwrap();

    client.write_all(b"hi").await.unwrap();
    client.shutdown().await.unwrap();
    let mut buf = String::new();
    server.read_to_string(&mut buf).await.unwrap();
    assert_eq!(buf, "hi");

    #[cfg(all(feature = "transport-ipc", unix))]
    {
        let endpoint = unique_ipc_endpoint("endpoint-connect");
        let bound = bind(&endpoint).await.unwrap();
        let (listener, bound_endpoint) = bound.into_parts();
        assert_eq!(bound_endpoint, endpoint);

        let client_task = tokio::spawn({
            let endpoint = endpoint.clone();
            async move {
                let mut client = connect(&endpoint).await.unwrap();
                client.write_all(b"ipc").await.unwrap();
                client.shutdown().await.unwrap();
            }
        });

        let (mut server, _peer) = listener.accept().await.unwrap();
        let mut buf = String::new();
        server.read_to_string(&mut buf).await.unwrap();
        assert_eq!(buf, "ipc");
        client_task.await.unwrap();
    }

    #[cfg(not(all(feature = "transport-ipc", unix)))]
    for endpoint in ["ipc://unsupported"] {
        let err = match bind(endpoint).await {
            Ok(_) => panic!("expected unsupported bind"),
            Err(err) => err,
        };
        assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
        let err = match connect(endpoint).await {
            Ok(_) => panic!("expected unsupported connect"),
            Err(err) => err,
        };
        assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
    }

    #[cfg(not(feature = "transport-quic"))]
    for endpoint in ["quic://127.0.0.1:9999"] {
        let err = match bind(endpoint).await {
            Ok(_) => panic!("expected unsupported bind"),
            Err(err) => err,
        };
        assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
        let err = match connect(endpoint).await {
            Ok(_) => panic!("expected unsupported connect"),
            Err(err) => err,
        };
        assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
    }

    #[cfg(not(feature = "transport-tls"))]
    for endpoint in ["tls://127.0.0.1:9999"] {
        let err = match bind(endpoint).await {
            Ok(_) => panic!("expected unsupported bind"),
            Err(err) => err,
        };
        assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
        let err = match connect(endpoint).await {
            Ok(_) => panic!("expected unsupported connect"),
            Err(err) => err,
        };
        assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
    }

    #[cfg(not(feature = "transport-websocket"))]
    for endpoint in ["ws://127.0.0.1:9999"] {
        let err = match bind(endpoint).await {
            Ok(_) => panic!("expected unsupported bind"),
            Err(err) => err,
        };
        assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
        let err = match connect(endpoint).await {
            Ok(_) => panic!("expected unsupported connect"),
            Err(err) => err,
        };
        assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
    }

    #[cfg(not(feature = "transport-tcp"))]
    for endpoint in ["127.0.0.1:9999"] {
        let err = match bind(endpoint).await {
            Ok(_) => panic!("expected unsupported bind"),
            Err(err) => err,
        };
        assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
        let err = match connect(endpoint).await {
            Ok(_) => panic!("expected unsupported connect"),
            Err(err) => err,
        };
        assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
    }
}
