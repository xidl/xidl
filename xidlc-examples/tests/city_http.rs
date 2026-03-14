use xidlc_examples::city_http::SmartCityHttpApiClient;
use xidlc_examples::city_http::SmartCityHttpApiServer;
use xidlc_examples::city_http::SmartCityHttpService;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_client_calls_all_endpoints() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let addr = listener.local_addr().expect("read local addr");
    drop(listener);

    let server_addr = addr.to_string();
    let task = tokio::spawn(async move {
        xidl_rust_axum::Server::builder()
            .with_service(SmartCityHttpApiServer::new(SmartCityHttpService))
            .serve(&server_addr)
            .await
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let base = format!("http://{}", addr);
    let client = SmartCityHttpApiClient::new(base);

    let eta = client
        .get_stop_eta("S100".to_string(), "2".to_string(), "en".to_string())
        .await
        .expect("call get_stop_eta");
    assert_eq!(eta.r#return, "S100");
    assert_eq!(eta.eta_seconds, 240);
    assert_eq!(eta.destination, "Central Station");

    let nearby = client
        .list_nearby_stops("S100".to_string())
        .await
        .expect("call list_nearby_stops");
    assert_eq!(nearby, vec!["S100-A".to_string(), "S100-B".to_string()]);

    let asset = client
        .download_asset("docs/readme.txt".to_string(), "v1".to_string())
        .await
        .expect("call download_asset");
    assert_eq!(
        asset.r#return,
        vec![
            97, 115, 115, 101, 116, 58, 100, 111, 99, 115, 47, 114, 101, 97, 100, 109, 101, 46,
            116, 120, 116
        ]
    );
    assert_eq!(asset.content_type, "text/plain");
    assert_eq!(asset.etag, "etag-demo");

    client
        .probe_lot("P1".to_string(), "t1".to_string())
        .await
        .expect("call probe_lot");

    let reserve = client
        .reserve_lot("P1".to_string(), "A-12345".to_string(), 30)
        .await
        .expect("call reserve_lot");
    assert_eq!(reserve.r#return, "resv-P1");
    assert_eq!(reserve.reservation_state, "CONFIRMED");
    assert_eq!(reserve.expires_at, "2026-03-08T10:00:00Z");

    client
        .cancel_reservation("P1".to_string(), "R1".to_string())
        .await
        .expect("call cancel_reservation");

    let profile = client
        .get_profile("C100".to_string())
        .await
        .expect("call get_profile");
    assert_eq!(profile.r#return, "C100");
    assert_eq!(profile.display_name, "Taylor");
    assert_eq!(profile.phone_number, "+1-555-0101");
    assert_eq!(profile.language, "en-US");

    let update = client
        .update_profile(
            "C100".to_string(),
            "Taylor".to_string(),
            "+1-555-0101".to_string(),
        )
        .await
        .expect("call update_profile");
    assert_eq!(update, "audit-20260307-001");

    let device = client
        .get_device_status(
            "D100".to_string(),
            "trace-001".to_string(),
            "sess-abc".to_string(),
            "en-US".to_string(),
        )
        .await
        .expect("call get_device_status");
    assert_eq!(device.r#return, "device:D100");
    assert_eq!(device.trace_echo, "trace-001");
    assert_eq!(device.session_echo, "sess-abc");

    let api_version = client.api_version().await.expect("call api_version");
    assert_eq!(api_version, "v2.0.0");

    let maintenance_mode = client
        .maintenance_mode()
        .await
        .expect("call maintenance_mode");
    assert!(!maintenance_mode);

    client
        .set_maintenance_mode(true)
        .await
        .expect("call set_maintenance_mode");

    task.abort();
}
