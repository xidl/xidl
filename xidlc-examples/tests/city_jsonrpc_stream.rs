use xidl_jsonrpc::futures_util::StreamExt;
use xidlc_examples::city_jsonrpc_stream::{
    CityJsonrpcStreamApi, CityJsonrpcStreamApiClient, CityJsonrpcStreamApiServer,
    CityJsonrpcStreamApichatParams, CityJsonrpcStreamService, Packet, PacketKind,
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

    let in_stream = xidl_jsonrpc::stream::boxed(async_stream::try_stream! {
        yield CityJsonrpcStreamApichatParams {
            room_id: "ops".to_string(),
            text: "hello".to_string(),
        };
        yield CityJsonrpcStreamApichatParams {
            room_id: "ops".to_string(),
            text: "world".to_string(),
        };
    });
    let mut chat = client.chat(in_stream).await.expect("open chat duplex");
    let first = chat
        .next()
        .await
        .expect("first chat item")
        .expect("first chat payload");
    assert_eq!(first.from, "server");
    assert_eq!(first.text, "echo:ops:hello");
    let second = chat
        .next()
        .await
        .expect("second chat item")
        .expect("second chat payload");
    assert_eq!(second.from, "server");
    assert_eq!(second.text, "echo:ops:world");

    let packet_in = xidl_jsonrpc::stream::boxed(async_stream::try_stream! {
        yield Packet::new_seq(10);
        yield Packet::new_payload("sensor-reading".to_string());
    });
    let mut packet_out = client
        .packet_stream(packet_in)
        .await
        .expect("open packet duplex");
    let p1 = packet_out
        .next()
        .await
        .expect("first packet item")
        .expect("first packet payload");
    assert_eq!(*p1.tag(), PacketKind::HEARTBEAT);
    assert_eq!(*p1.as_seq(), 11);
    let p2 = packet_out
        .next()
        .await
        .expect("second packet item")
        .expect("second packet payload");
    assert_eq!(*p2.tag(), PacketKind::DATA);
    assert_eq!(p2.as_payload(), "echo:sensor-reading");

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
    assert_eq!(first_alert, "pudong:alert-0");
    assert_eq!(second_alert, "pudong:alert-1");

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
    assert_eq!(first_alert, "pudong:alert-0");

    let in_stream = xidl_jsonrpc::stream::boxed(async_stream::try_stream! {
        yield CityJsonrpcStreamApichatParams {
            room_id: "ops".to_string(),
            text: "inproc".to_string(),
        };
    });
    let mut chat = client.chat(in_stream).await.expect("open chat duplex");
    let reply = chat
        .next()
        .await
        .expect("chat reply item")
        .expect("chat reply payload");
    assert_eq!(reply.from, "server");
    assert_eq!(reply.text, "echo:ops:inproc");

    let packet_in = xidl_jsonrpc::stream::boxed(async_stream::try_stream! {
        yield Packet::new_seq(99);
    });
    let mut packet_out = client
        .packet_stream(packet_in)
        .await
        .expect("open inproc packet duplex");
    let p = packet_out
        .next()
        .await
        .expect("inproc packet item")
        .expect("inproc packet payload");
    assert_eq!(*p.tag(), PacketKind::HEARTBEAT);
    assert_eq!(*p.as_seq(), 100);

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
