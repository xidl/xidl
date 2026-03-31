use xidlc_examples::hysteria2::Hysteria2Client;
use xidlc_examples::hysteria2::Hysteria2Server;
use xidlc_examples::hysteria2::ImHysteria2Server;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hysteria2_auth_uses_headers_for_request_and_response() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let addr = listener.local_addr().expect("read local addr");
    drop(listener);

    let server_addr = addr.to_string();
    let task = tokio::spawn(async move {
        xidl_rust_axum::Server::builder()
            .with_service(Hysteria2Server::new(ImHysteria2Server))
            .serve(&server_addr)
            .await
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let client = Hysteria2Client::new(format!("http://{}", addr));
    let response = client
        .auth(
            "user:password".to_string(),
            114_514,
            "random-padding".to_string(),
        )
        .await
        .expect("call hysteria2 auth");

    assert!(response.udp);
    assert_eq!(response.rx, 114_514);
    assert_eq!(response.padding, "random-padding");

    task.abort();
}
