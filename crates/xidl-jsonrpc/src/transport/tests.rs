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

#[cfg(all(feature = "transport-ipc", unix))]
#[tokio::test]
async fn ipc_bind_rejects_live_listener_and_reclaims_stale_socket() {
    let endpoint = unique_ipc_endpoint("bind");
    let path = endpoint.trim_start_matches("ipc://");

    // A live listener owns the path; a second bind must fail with AddrInUse.
    let listener = super::IpcListener::bind(path).unwrap();
    let err = match super::IpcListener::bind(path) {
        Ok(_) => panic!("expected second bind to fail"),
        Err(err) => err,
    };
    assert_eq!(err.kind(), std::io::ErrorKind::AddrInUse);
    drop(listener);

    // A leftover file with no live listener is reclaimed and bound.
    std::fs::write(path, b"stale").unwrap();
    let reclaimed = super::IpcListener::bind(path).unwrap();
    assert_eq!(reclaimed.endpoint().as_deref(), Some(endpoint.as_str()));
    drop(reclaimed);
}

#[cfg(all(feature = "transport-ipc", unix))]
#[test]
fn ipc_bind_rejects_oversized_path() {
    let long_path = format!("/tmp/{}", "x".repeat(300));
    let err = match super::IpcListener::bind(&long_path) {
        Ok(_) => panic!("expected oversized path to fail"),
        Err(err) => err,
    };
    assert_ne!(err.kind(), std::io::ErrorKind::AddrInUse);
}

#[cfg(all(feature = "transport-ipc", unix))]
#[tokio::test]
async fn ipc_drop_tolerates_missing_socket_file() {
    let endpoint = unique_ipc_endpoint("drop-missing");
    let path = endpoint.trim_start_matches("ipc://");
    let listener = super::IpcListener::bind(path).unwrap();
    std::fs::remove_file(path).unwrap();
    drop(listener);
}

#[cfg(all(feature = "transport-ipc", unix))]
#[tokio::test]
async fn ipc_drop_tolerates_replaced_socket_file() {
    let endpoint = unique_ipc_endpoint("drop-dir");
    let path = endpoint.trim_start_matches("ipc://");
    let listener = super::IpcListener::bind(path).unwrap();
    std::fs::remove_file(path).unwrap();
    std::fs::create_dir(path).unwrap();
    drop(listener);
    std::fs::remove_dir(path).unwrap();
}

#[cfg(any(feature = "transport-tls", feature = "transport-websocket"))]
#[test]
fn tls_config_rejects_unsupported_scheme_and_missing_parts() {
    let err = super::tls_config::TransportUrl::parse("tcp://127.0.0.1:1", &["tls"])
        .err()
        .expect("unsupported scheme");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert!(err.to_string().contains("unsupported scheme"));

    let no_host = super::tls_config::TransportUrl::parse("tls:foo", &["tls"]).expect("parse");
    let err = no_host.host_port().expect_err("missing host");
    assert!(err.to_string().contains("missing host"));

    let no_port =
        super::tls_config::TransportUrl::parse("tls://localhost", &["tls"]).expect("parse");
    let err = no_port.host_port().expect_err("missing port");
    assert!(err.to_string().contains("missing port"));
}

#[cfg(any(feature = "transport-tls", feature = "transport-websocket"))]
#[test]
fn tls_config_wraps_ipv6_hosts_in_socket_bind_addr() {
    assert_eq!(
        super::tls_config::socket_bind_addr("::1", 8443),
        "[::1]:8443"
    );
}

#[cfg(feature = "transport-websocket")]
async fn free_ws_port() -> u16 {
    let probe = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = probe.local_addr().unwrap().port();
    drop(probe);
    port
}

#[cfg(feature = "transport-websocket")]
#[tokio::test]
async fn websocket_round_trip_exchanges_text_and_binary_frames() {
    let port = free_ws_port().await;
    let endpoint = format!("ws://127.0.0.1:{port}");
    let listener = super::websocket::WebSocketListener::bind(&endpoint)
        .await
        .unwrap();
    listener.set_frame_kind(crate::FrameKind::Binary);

    let server_task = tokio::spawn(async move {
        let (mut stream, _peer) = listener.accept().await.unwrap();
        let mut buf = [0_u8; 4];
        stream.read_exact(&mut buf).await.unwrap();
        stream.write_all(b"pong").await.unwrap();
        stream.flush().await.unwrap();
    });

    let mut client = super::websocket::connect_websocket(&endpoint)
        .await
        .unwrap();
    client.write_all(b"ping").await.unwrap();
    client.flush().await.unwrap();
    let mut echo = [0_u8; 4];
    client.read_exact(&mut echo).await.unwrap();
    assert_eq!(&echo, b"pong");
    client.shutdown().await.unwrap();
    server_task.await.unwrap();
}

#[cfg(feature = "transport-websocket")]
#[tokio::test]
async fn wss_round_trip_with_self_signed_cert() {
    let probe = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = probe.local_addr().unwrap().port();
    drop(probe);

    let bind_url = format!(
        "wss://localhost:{port}?cert=/tmp/xidl-server-cert.pem&key=/tmp/xidl-server-key.pem"
    );
    let listener = super::websocket::WebSocketListener::bind(&bind_url)
        .await
        .unwrap();

    let server_task = tokio::spawn(async move {
        let (mut stream, _peer) = listener.accept().await.unwrap();
        let mut buf = [0_u8; 4];
        stream.read_exact(&mut buf).await.unwrap();
        stream.write_all(b"pong").await.unwrap();
        stream.flush().await.unwrap();
    });

    let connect_url = format!("wss://localhost:{port}?ca=/tmp/xidl-ca-cert.pem");
    let mut client = super::websocket::connect_websocket(&connect_url)
        .await
        .unwrap();
    client.write_all(b"ping").await.unwrap();
    client.flush().await.unwrap();
    let mut echo = [0_u8; 4];
    client.read_exact(&mut echo).await.unwrap();
    assert_eq!(&echo, b"pong");
    client.shutdown().await.unwrap();
    server_task.await.unwrap();
}
