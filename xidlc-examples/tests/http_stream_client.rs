use xidlc_examples::city_http_stream::CityHttpStreamApiChatRequest;
use xidlc_examples::city_http_stream::CityHttpStreamApiClient;
use xidlc_examples::city_http_stream::CityHttpStreamApiServer;
use xidlc_examples::city_http_stream::CityHttpStreamApiUploadAssetRequest;
use xidlc_examples::example_services::CityHttpStreamService;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_stream_client_calls_stream_endpoints() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let addr = listener.local_addr().expect("read local addr");
    drop(listener);

    let server_addr = addr.to_string();
    let task = tokio::spawn(async move {
        xidl_rust_axum::Server::builder()
            .with_service(CityHttpStreamApiServer::new(CityHttpStreamService))
            .serve(&server_addr)
            .await
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let base = format!("http://{}", addr);
    let client = CityHttpStreamApiClient::new(base);

    let mut alerts = client
        .alerts("pudong".to_string(), "en".to_string())
        .await
        .expect("call alerts");
    let first = alerts
        .read()
        .await
        .expect("first alert")
        .expect("first alert payload");
    let second = alerts
        .read()
        .await
        .expect("second alert")
        .expect("second alert payload");
    assert_eq!(first, "pudong:ALERT:1:en");
    assert_eq!(second, "pudong:ALERT:2:en");

    let mut ticker = client.ticker().await.expect("call ticker");
    let t1 = ticker
        .read()
        .await
        .expect("first ticker")
        .expect("first ticker payload");
    let t2 = ticker
        .read()
        .await
        .expect("second ticker")
        .expect("second ticker payload");
    assert_eq!(t1, "tick-1");
    assert_eq!(t2, "tick-2");

    let mut upload = client.upload_asset().await.expect("open upload_asset");
    upload
        .write(CityHttpStreamApiUploadAssetRequest {
            asset_id: "asset-1".to_string(),
            chunk: vec![1, 2, 3],
        })
        .await
        .expect("write first chunk");
    upload
        .write(CityHttpStreamApiUploadAssetRequest {
            asset_id: "asset-1".to_string(),
            chunk: vec![4, 5],
        })
        .await
        .expect("write second chunk");
    let upload_resp = upload.close().await.expect("close upload_asset");
    assert_eq!(upload_resp, "uploaded:asset-1:5");

    let mut chat = client.chat().await.expect("open chat");
    chat.write(CityHttpStreamApiChatRequest {
        room: "ops".to_string(),
        message: "hello".to_string(),
    })
    .await
    .expect("write chat request");
    let reply = chat
        .read()
        .await
        .expect("chat reply item")
        .expect("chat reply payload");
    assert_eq!(reply.from, "server");
    assert_eq!(reply.text, "echo:ops:hello");
    chat.close();

    task.abort();
}
