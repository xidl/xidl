use xidlc_examples::hysteria2::Hysteria2Client;
use xidlc_examples::hysteria2::Hysteria2Server;
use xidlc_examples::hysteria2::ImHysteria2Server;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hysteria2_auth_uses_headers_for_request_and_response() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let addr = listener.local_addr().expect("read local addr");
    listener
        .set_nonblocking(true)
        .expect("set listener nonblocking");
    let listener = tokio::net::TcpListener::from_std(listener).expect("adopt listener for tokio");

    let task = tokio::spawn(async move {
        xidl_rust_axum::Server::builder()
            .with_service(Hysteria2Server::new(ImHysteria2Server))
            .serve_with_listener(listener)
            .await
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let http = xidl_rust_axum::reqwest::Client::builder()
        .build()
        .expect("build reqwest client without proxy");
    let client = Hysteria2Client::with_http(format!("http://{}", addr), http);
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
