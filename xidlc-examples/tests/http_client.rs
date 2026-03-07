use xidlc_examples::city_http::SmartCityHttpApiServer;
use xidlc_examples::example_services::SmartCityHttpService;

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
    let client = xidl_rust_axum::reqwest::Client::new();

    let eta = client
        .get(format!("{base}/v1/stops/S100?line=2&lang=en"))
        .send()
        .await
        .expect("call get_stop_eta");
    assert_eq!(eta.status(), xidl_rust_axum::reqwest::StatusCode::OK);
    let eta_json = eta
        .json::<serde_json::Value>()
        .await
        .expect("decode get_stop_eta");
    assert_eq!(
        eta_json,
        serde_json::json!({
            "return": "S100",
            "eta_seconds": 240,
            "destination": "Central Station"
        })
    );

    let nearby = client
        .get(format!("{base}/v1/stops/S100/nearby"))
        .send()
        .await
        .expect("call list_nearby_stops");
    assert_eq!(nearby.status(), xidl_rust_axum::reqwest::StatusCode::OK);
    let nearby_json = nearby
        .json::<serde_json::Value>()
        .await
        .expect("decode nearby");
    assert_eq!(nearby_json, serde_json::json!(["S100-A", "S100-B"]));

    let asset = client
        .get(format!("{base}/v1/assets/docs/readme.txt?version=v1"))
        .send()
        .await
        .expect("call download_asset");
    assert_eq!(asset.status(), xidl_rust_axum::reqwest::StatusCode::OK);
    let asset_json = asset
        .json::<serde_json::Value>()
        .await
        .expect("decode download_asset");
    assert_eq!(
        asset_json,
        serde_json::json!({
            "return": [97, 115, 115, 101, 116, 58, 100, 111, 99, 115, 47, 114, 101, 97, 100, 109, 101, 46, 116, 120, 116],
            "content_type": "text/plain",
            "etag": "etag-demo"
        })
    );

    let probe = client
        .head(format!("{base}/v1/parking/lots/P1?trace_id=t1"))
        .send()
        .await
        .expect("call probe_lot");
    assert_eq!(
        probe.status(),
        xidl_rust_axum::reqwest::StatusCode::NO_CONTENT
    );

    let reserve = client
        .post(format!("{base}/v1/parking/lots/P1/reserve"))
        .json(&serde_json::json!({ "plate_number": "A-12345", "minutes": 30 }))
        .send()
        .await
        .expect("call reserve_lot");
    assert_eq!(reserve.status(), xidl_rust_axum::reqwest::StatusCode::OK);
    let reserve_json = reserve
        .json::<serde_json::Value>()
        .await
        .expect("decode reserve_lot");
    assert_eq!(
        reserve_json,
        serde_json::json!({
            "return": "resv-P1",
            "reservation_state": "CONFIRMED",
            "expires_at": "2026-03-08T10:00:00Z"
        })
    );

    let cancel = client
        .delete(format!("{base}/v1/parking/lots/P1/reservations/R1"))
        .send()
        .await
        .expect("call cancel_reservation");
    assert_eq!(
        cancel.status(),
        xidl_rust_axum::reqwest::StatusCode::NO_CONTENT
    );

    let profile = client
        .get(format!("{base}/v1/citizens/C100"))
        .send()
        .await
        .expect("call get_profile");
    assert_eq!(profile.status(), xidl_rust_axum::reqwest::StatusCode::OK);
    let profile_json = profile
        .json::<serde_json::Value>()
        .await
        .expect("decode get_profile");
    assert_eq!(
        profile_json,
        serde_json::json!({
            "return": "C100",
            "display_name": "Taylor",
            "phone_number": "+1-555-0101",
            "language": "en-US"
        })
    );

    let update = client
        .patch(format!("{base}/v1/citizens/C100"))
        .json(&serde_json::json!({ "display_name": "Taylor", "phone_number": "+1-555-0101" }))
        .send()
        .await
        .expect("call update_profile");
    assert_eq!(update.status(), xidl_rust_axum::reqwest::StatusCode::OK);
    let update_json = update
        .json::<serde_json::Value>()
        .await
        .expect("decode update_profile");
    assert_eq!(update_json, serde_json::json!("audit-20260307-001"));

    let api_version = client
        .get(format!("{base}/SmartCityHttpApi/api_version"))
        .send()
        .await
        .expect("call api_version");
    assert_eq!(
        api_version.status(),
        xidl_rust_axum::reqwest::StatusCode::OK
    );
    let api_version_json = api_version
        .json::<serde_json::Value>()
        .await
        .expect("decode api_version");
    assert_eq!(api_version_json, serde_json::json!("v2.0.0"));

    let maintenance_mode = client
        .get(format!("{base}/SmartCityHttpApi/maintenance_mode"))
        .send()
        .await
        .expect("call maintenance_mode");
    assert_eq!(
        maintenance_mode.status(),
        xidl_rust_axum::reqwest::StatusCode::OK
    );
    let maintenance_mode_json = maintenance_mode
        .json::<serde_json::Value>()
        .await
        .expect("decode maintenance_mode");
    assert_eq!(maintenance_mode_json, serde_json::json!(false));

    let set_maintenance_mode = client
        .post(format!("{base}/SmartCityHttpApi/set_maintenance_mode"))
        .json(&serde_json::json!(true))
        .send()
        .await
        .expect("call set_maintenance_mode");
    assert_eq!(
        set_maintenance_mode.status(),
        xidl_rust_axum::reqwest::StatusCode::NO_CONTENT
    );

    task.abort();
}
