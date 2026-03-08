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
        .watch_alert("pudong".to_string(), "en".to_string())
        .await
        .expect("call watch_alert");
    let first = xidl_rust_axum::futures_util::StreamExt::next(&mut alerts)
        .await
        .expect("first alert")
        .expect("first alert payload");
    let second = xidl_rust_axum::futures_util::StreamExt::next(&mut alerts)
        .await
        .expect("second alert")
        .expect("second alert payload");
    assert_eq!(first, "pudong:ALERT:1:en");
    assert_eq!(second, "pudong:ALERT:2:en");

    let mut attr = client
        .watch_attribute_maintenance_mode()
        .await
        .expect("call watch_attribute_maintenance_mode");
    let m1 = xidl_rust_axum::futures_util::StreamExt::next(&mut attr)
        .await
        .expect("first attr")
        .expect("first attr payload");
    let m2 = xidl_rust_axum::futures_util::StreamExt::next(&mut attr)
        .await
        .expect("second attr")
        .expect("second attr payload");
    assert!(!m1);
    assert!(m2);

    let upload_stream =
        xidl_rust_axum::stream::boxed_ndjson(xidl_rust_axum::futures_util::stream::iter(vec![
            Ok(CityHttpStreamApiUploadAssetRequest {
                asset_id: "asset-1".to_string(),
                chunk: vec![1, 2, 3],
            }),
            Ok(CityHttpStreamApiUploadAssetRequest {
                asset_id: "asset-1".to_string(),
                chunk: vec![4, 5],
            }),
        ]));
    let upload_resp = client
        .upload_asset(upload_stream)
        .await
        .expect("call upload_asset");
    assert_eq!(upload_resp, "uploaded:asset-1:5");

    task.abort();
}
