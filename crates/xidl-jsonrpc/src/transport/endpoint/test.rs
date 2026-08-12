use super::Endpoint;

#[test]
fn parse_falls_back_to_tcp_when_scheme_is_missing() {
    match Endpoint::parse("127.0.0.1:7000") {
        Endpoint::Tcp(addr) => assert_eq!(addr, "127.0.0.1:7000"),
        _ => panic!("expected tcp endpoint"),
    }
}

#[test]
fn parse_strips_tcp_scheme_prefix() {
    match Endpoint::parse("tcp://127.0.0.1:7001") {
        Endpoint::Tcp(addr) => assert_eq!(addr, "127.0.0.1:7001"),
        _ => panic!("expected tcp endpoint"),
    }
}

#[test]
fn parse_preserves_quic_tls_and_websocket_schemes() {
    match Endpoint::parse("quic://127.0.0.1:7002") {
        Endpoint::Quic(endpoint) => assert_eq!(endpoint, "quic://127.0.0.1:7002"),
        _ => panic!("expected quic endpoint"),
    }
    match Endpoint::parse("tls://127.0.0.1:7003") {
        Endpoint::Tls(endpoint) => assert_eq!(endpoint, "tls://127.0.0.1:7003"),
        _ => panic!("expected tls endpoint"),
    }
    match Endpoint::parse("wss://127.0.0.1:7004/rpc") {
        Endpoint::WebSocket(endpoint) => assert_eq!(endpoint, "wss://127.0.0.1:7004/rpc"),
        _ => panic!("expected websocket endpoint"),
    }
}

#[tokio::test]
async fn bind_and_connect_quic_cover_feature_enabled_dispatch() {
    let bind_err = super::bind("quic://127.0.0.1:7005")
        .await
        .err()
        .expect("quic bind error");
    assert!(!bind_err.to_string().is_empty());

    let connect_err = super::connect("quic://127.0.0.1:7005")
        .await
        .err()
        .expect("quic connect error");
    assert!(!connect_err.to_string().is_empty());
}

#[test]
fn parse_marks_unknown_schemes_as_unsupported() {
    match Endpoint::parse("foo://bar") {
        Endpoint::Unsupported(endpoint) => assert_eq!(endpoint, "foo://bar"),
        _ => panic!("expected unsupported endpoint"),
    }
}

#[tokio::test]
async fn bind_and_connect_report_unsupported_schemes_as_invalid_input() {
    let bind_err = super::bind("foo://bar")
        .await
        .err()
        .expect("unsupported bind should fail");
    assert_eq!(bind_err.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(bind_err.to_string(), "unsupported scheme: foo://bar");

    let connect_err = super::connect("foo://bar")
        .await
        .err()
        .expect("unsupported connect should fail");
    assert_eq!(connect_err.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(connect_err.to_string(), "unsupported scheme: foo://bar");
}
