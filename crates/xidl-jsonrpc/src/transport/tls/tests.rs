use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn free_tcp_port() -> u16 {
    let probe = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("probe bind");
    let port = probe.local_addr().expect("local addr").port();
    drop(probe);
    port
}

#[tokio::test]
async fn tls_round_trip_exchanges_data_over_encrypted_stream() {
    let port = free_tcp_port().await;
    let bind_url = format!(
        "tls://127.0.0.1:{port}?cert=/tmp/xidl-server-cert.pem&key=/tmp/xidl-server-key.pem"
    );
    let listener = TlsListener::bind(&bind_url).await.expect("tls bind");

    let server_task = tokio::spawn(async move {
        let (mut stream, _peer) = listener.accept().await.expect("tls accept");
        let mut buf = [0_u8; 4];
        stream.read_exact(&mut buf).await.expect("server read");
        stream.write_all(&buf).await.expect("server write");
        stream.flush().await.expect("server flush");
    });

    let connect_url =
        format!("tls://127.0.0.1:{port}?ca=/tmp/xidl-ca-cert.pem&server_name=localhost");
    let mut client = connect_tls(&connect_url).await.expect("tls connect");
    client.write_all(b"ping").await.expect("client write");
    client.flush().await.expect("client flush");

    let mut echo = [0_u8; 4];
    client.read_exact(&mut echo).await.expect("client read");
    assert_eq!(&echo, b"ping");

    client.shutdown().await.expect("client shutdown");
    server_task.await.expect("server task");
}

#[tokio::test]
async fn tls_bind_and_connect_validate_required_parameters_before_io() {
    let bind_err = TlsListener::bind("tls://127.0.0.1:8443")
        .await
        .err()
        .expect("missing cert/key");
    assert!(
        bind_err
            .to_string()
            .contains("missing tls parameter `cert`")
    );

    let connect_err = connect_tls("tls://127.0.0.1:8443")
        .await
        .err()
        .expect("missing ca");
    assert!(
        connect_err
            .to_string()
            .contains("missing tls parameter `ca`")
    );
}

#[tokio::test]
async fn tls_connect_validates_server_name_and_acceptor_inputs() {
    let err = connect_tls("tls://127.0.0.1:8443?ca=/tmp/does-not-exist&server_name=bad name")
        .await
        .err()
        .expect("invalid server name");
    assert!(!err.to_string().is_empty());

    let err = TlsListener::bind("tls://127.0.0.1:8443?cert=/tmp/missing-cert&key=/tmp/missing-key")
        .await
        .err()
        .expect("invalid cert path");
    assert!(!err.to_string().is_empty());
}
