use xidlc_examples::city_jsonrpc::{
    SmartCityRpcApi, SmartCityRpcApiClient, SmartCityRpcApiServer, SmartCityRpcService,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn jsonrpc_client_calls_all_endpoints() {
    let server = xidl_jsonrpc::Server::builder()
        .with_service(SmartCityRpcApiServer::new(SmartCityRpcService::default()))
        .with_endpoint("tcp://127.0.0.1:0")
        .build()
        .await
        .expect("build server");
    let endpoint = server.endpoint().expect("server endpoint").to_string();
    let task = tokio::spawn(async move { server.serve().await });

    let client = SmartCityRpcApiClient::builder()
        .with_endpoint(endpoint)
        .build()
        .await
        .expect("build client");

    let quote = client
        .quote_trip("rider-1".to_string(), "zone-a".to_string())
        .await
        .expect("call quote_trip");
    assert_eq!(quote.r#return, "quote-rider-1-zone-a");
    assert_eq!(quote.amount_cents, 1880);
    assert_eq!(quote.currency, "CNY");

    let invoice = client
        .create_invoice("rider-1".to_string(), 1880, "CNY".to_string())
        .await
        .expect("call create_invoice");
    assert_eq!(invoice.r#return, "created");
    assert_eq!(invoice.invoice_id, "inv-rider-1-1880");
    assert_eq!(
        invoice.payment_url,
        "https://pay.example.com/inv-rider-1-1880?ccy=CNY"
    );

    client
        .mark_paid(invoice.invoice_id.clone())
        .await
        .expect("call mark_paid");
    client.heartbeat().await.expect("call heartbeat");

    let rotated = client
        .rotate_session("token-1".to_string())
        .await
        .expect("call rotate_session");
    assert_eq!(rotated.session_token, "token-1-next");
    assert_eq!(rotated.expires_at_epoch_sec, 1_710_000_000);

    let dispatch = client
        .dispatch("vehicle-7".to_string(), "stop-2".to_string())
        .await
        .expect("call dispatch");
    assert_eq!(dispatch.r#return, 12);
    assert_eq!(dispatch.job_id, "job-vehicle-7-stop-2");

    client
        .report_trip(
            "order-1".to_string(),
            "rider-1".to_string(),
            "done".to_string(),
        )
        .await
        .expect("call report_trip");

    let summary = client
        .summarize("2026-03-07".to_string())
        .await
        .expect("call summarize");
    assert_eq!(summary.trip_count, 42);
    assert_eq!(summary.gross_revenue_cents, 123_456);

    let region = client
        .get_attribute_region()
        .await
        .expect("call get_attribute_region");
    assert_eq!(region, "cn-east");

    let channel = client
        .get_attribute_firmware_channel()
        .await
        .expect("call get_attribute_firmware_channel");
    assert_eq!(channel, "stable");

    client
        .set_attribute_firmware_channel("canary".to_string())
        .await
        .expect("call set_attribute_firmware_channel");
    let updated_channel = client
        .get_attribute_firmware_channel()
        .await
        .expect("call get_attribute_firmware_channel after set");
    assert_eq!(updated_channel, "canary");

    task.abort();
}
