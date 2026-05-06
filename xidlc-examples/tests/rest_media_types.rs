use xidlc_examples::rest_media_types::RestMediaTypesApiClient;
use xidlc_examples::rest_media_types::RestMediaTypesApiServer;
use xidlc_examples::rest_media_types::RestMediaTypesService;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_client_supports_form_and_msgpack_media_types() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let addr = listener.local_addr().expect("read local addr");
    listener
        .set_nonblocking(true)
        .expect("set listener nonblocking");
    let listener = tokio::net::TcpListener::from_std(listener).expect("adopt listener for tokio");

    let task = tokio::spawn(async move {
        xidl_rust_axum::Server::builder()
            .with_service(RestMediaTypesApiServer::new(RestMediaTypesService))
            .serve_with_listener(listener)
            .await
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let http = xidl_rust_axum::reqwest::Client::builder()
        .build()
        .expect("build reqwest client without proxy");
    let client = RestMediaTypesApiClient::with_http(format!("http://{}", addr), http);

    let submit = client
        .submit_profile("Taylor".to_string(), 42)
        .await
        .expect("submit profile");
    assert_eq!(submit.r#return, "Taylor:42");
    assert_eq!(submit.normalized_name, "TAYLOR");

    let msgpack = client
        .get_msgpack_user("u100".to_string())
        .await
        .expect("get msgpack user");
    assert_eq!(msgpack.r#return, "user:u100");
    assert_eq!(msgpack.score, 95);

    task.abort();
}
