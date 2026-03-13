use xidl_jsonrpc::futures_util::StreamExt;
use xidlc_examples::city_jsonrpc_stream::{
    CityJsonrpcStreamApi, CityJsonrpcStreamApiClient, CityJsonrpcStreamApiServer,
    CityJsonrpcStreamService,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn jsonrpc_client_calls_stream_endpoints() {
    let server = xidl_jsonrpc::Server::builder()
        .with_service(CityJsonrpcStreamApiServer::new(CityJsonrpcStreamService))
        .with_endpoint("tcp://127.0.0.1:0")
        .build()
        .await
        .expect("build server");
    let endpoint = server.endpoint().expect("server endpoint").to_string();
    let task = tokio::spawn(async move { server.serve().await });

    let client = CityJsonrpcStreamApiClient::builder()
        .with_endpoint(endpoint)
        .build()
        .await
        .expect("build client");

    let mut upload = client
        .upload_sensor()
        .await
        .expect("open upload sensor writer");
    upload
        .write(serde_json::json!({ "sensor_id": "sensor-1", "value": 42 }))
        .await
        .expect("write upload chunk");
    upload.close().await.expect("close upload sensor writer");

    let mut chat = client.chat().await.expect("open chat duplex");
    chat.write(serde_json::json!({ "room_id": "ops", "text": "hello" }))
        .await
        .expect("write chat item");
    chat.write(serde_json::json!({ "room_id": "ops", "text": "world" }))
        .await
        .expect("write second chat item");
    let first = chat
        .read()
        .await
        .expect("first chat item")
        .expect("first chat payload");
    assert_eq!(first["from"], "server");
    assert_eq!(first["text"], "echo:ops:hello");
    let second = chat
        .read()
        .await
        .expect("second chat item")
        .expect("second chat payload");
    assert_eq!(second["from"], "server");
    assert_eq!(second["text"], "echo:ops:world");
    chat.close().await.expect("close chat duplex");

    let mut alerts = client
        .alerts("pudong".to_string())
        .await
        .expect("call alerts");
    let first_alert = alerts
        .next()
        .await
        .expect("first alert item")
        .expect("first alert payload");
    let second_alert = alerts
        .next()
        .await
        .expect("second alert item")
        .expect("second alert payload");
    assert_eq!(first_alert["message"], "pudong:alert-0");
    assert_eq!(second_alert["message"], "pudong:alert-1");

    let mut notice_stream = client
        .get_attribute_ops_notice()
        .await
        .expect("call get_attribute_ops_notice");
    let first = notice_stream
        .read()
        .await
        .expect("first attribute item")
        .expect("first attribute payload");
    assert_eq!(first, "notice-1");
    let second = notice_stream
        .read()
        .await
        .expect("second attribute item")
        .expect("second attribute payload");
    assert_eq!(second, "notice-2");

    task.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn jsonrpc_bidi_stream_works_over_inproc_transport() {
    let endpoint = "city-jsonrpc-stream-bidi-inproc";
    let serve_endpoint = format!("inproc://{endpoint}");
    let server = xidl_jsonrpc::Server::builder()
        .with_service(CityJsonrpcStreamApiServer::new(CityJsonrpcStreamService))
        .with_endpoint(serve_endpoint)
        .build()
        .await
        .expect("build inproc server");
    let client_endpoint = server.endpoint().expect("server endpoint").to_string();
    let task = tokio::spawn(async move { server.serve().await });

    let client = CityJsonrpcStreamApiClient::builder()
        .with_endpoint(client_endpoint)
        .build()
        .await
        .expect("build client");

    let mut upload = client
        .upload_sensor()
        .await
        .expect("open upload sensor writer");
    upload
        .write(serde_json::json!({ "sensor_id": "sensor-inproc", "value": 7 }))
        .await
        .expect("write upload chunk");
    upload.close().await.expect("close upload sensor writer");

    let mut alerts = client
        .alerts("pudong".to_string())
        .await
        .expect("call alerts");
    let first_alert = alerts
        .next()
        .await
        .expect("first alert item")
        .expect("first alert payload");
    assert_eq!(first_alert["message"], "pudong:alert-0");

    let mut chat = client.chat().await.expect("open chat duplex");
    chat.write(serde_json::json!({ "room_id": "ops", "text": "inproc" }))
        .await
        .expect("write chat item");
    let reply = chat
        .read()
        .await
        .expect("chat reply item")
        .expect("chat reply payload");
    assert_eq!(reply["from"], "server");
    assert_eq!(reply["text"], "echo:ops:inproc");
    chat.close().await.expect("close chat duplex");

    let mut notice_stream = client
        .get_attribute_ops_notice()
        .await
        .expect("call get_attribute_ops_notice");
    let first_notice = notice_stream
        .read()
        .await
        .expect("first notice item")
        .expect("first notice payload");
    assert_eq!(first_notice, "notice-1");

    task.abort();
}
