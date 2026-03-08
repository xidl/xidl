use tokio::io::split;
use tokio::net::TcpStream;
use xidlc_examples::city_jsonrpc_stream::{
    CityJsonrpcStreamApi, CityJsonrpcStreamApiClient, CityJsonrpcStreamApiServer,
    CityJsonrpcStreamService,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn jsonrpc_client_calls_stream_endpoints() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let addr = listener.local_addr().expect("read local addr");
    drop(listener);

    let server_addr = addr.to_string();
    let task = tokio::spawn(async move {
        xidl_jsonrpc::Server::builder()
            .with_service(CityJsonrpcStreamApiServer::new(CityJsonrpcStreamService))
            .serve_on(&server_addr)
            .await
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let stream = TcpStream::connect(addr).await.expect("connect jsonrpc");
    stream.set_nodelay(true).expect("set nodelay");
    let (reader, writer) = split(stream);
    let client = CityJsonrpcStreamApiClient::new(reader, writer);

    let upload = async_stream::try_stream! {
        yield serde_json::json!({ "sensor_id": "sensor-1", "value": 42 });
        yield serde_json::json!({ "sensor_id": "sensor-1", "value": 43 });
    };
    let upload = xidl_jsonrpc::stream::boxed(upload);
    let _ = client.upload_sensor(upload).await;

    let chat_in = async_stream::try_stream! {
        yield serde_json::json!({ "room_id": "ops", "text": "hello" });
    };
    let chat_in = xidl_jsonrpc::stream::boxed(chat_in);
    let _ = client.chat(chat_in).await;

    let _ = client.subscribe_alert("pudong".to_string()).await;

    let mut notice_stream = client
        .get_attribute_ops_notice()
        .await
        .expect("call get_attribute_ops_notice");
    let first = xidl_rust_axum::futures_util::StreamExt::next(&mut notice_stream)
        .await
        .expect("first attribute item")
        .expect("first attribute payload");
    assert_eq!(first, "ok");

    task.abort();
}
