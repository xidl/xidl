use xidlc_examples::http_media_types::HttpMediaTypesApiClient;
use xidlc_examples::http_media_types::HttpMediaTypesApiServer;
use xidlc_examples::http_media_types::HttpMediaTypesService;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_client_supports_form_and_msgpack_media_types() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let addr = listener.local_addr().expect("read local addr");
    drop(listener);

    let server_addr = addr.to_string();
    let task = tokio::spawn(async move {
        xidl_rust_axum::Server::builder()
            .with_service(HttpMediaTypesApiServer::new(HttpMediaTypesService))
            .serve(&server_addr)
            .await
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let client = HttpMediaTypesApiClient::new(format!("http://{}", addr));

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
