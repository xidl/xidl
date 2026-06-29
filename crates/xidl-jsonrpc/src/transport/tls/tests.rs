use super::*;

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
