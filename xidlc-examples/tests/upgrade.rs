use tokio::io::{AsyncReadExt, AsyncWriteExt};
use xidlc_examples::upgrade::{
    HttpUpgradeService, HttpUpgradeServiceClient, HttpUpgradeServiceServer,
};

struct ImUpgradeServer;

#[async_trait::async_trait]
impl HttpUpgradeService for ImUpgradeServer {
    async fn connect_raw(
        &self,
        token: String,
        upgrade: xidl_rust_axum::upgrade::Upgrade,
    ) -> xidl_rust_axum::Result<xidl_rust_axum::axum::response::Response> {
        if token == "invalid" {
            return Err(xidl_rust_axum::Error::new(401, "invalid token"));
        }

        let response = upgrade.on_upgrade(|mut stream| async move {
            let mut buf = [0u8; 1024];
            if let Ok(n) = stream.read(&mut buf).await {
                if n > 0 {
                    let received = &buf[..n];
                    let mut response_buf = b"hello: ".to_vec();
                    response_buf.extend_from_slice(received);
                    let _ = stream.write_all(&response_buf).await;
                }
            }
        });

        Ok(response)
    }

    async fn connect_echo(
        &self,
        upgrade: xidl_rust_axum::upgrade::Upgrade,
    ) -> xidl_rust_axum::Result<xidl_rust_axum::axum::response::Response> {
        let response = upgrade.on_upgrade(|mut stream| async move {
            let mut buf = [0u8; 1024];
            while let Ok(n) = stream.read(&mut buf).await {
                if n == 0 {
                    break;
                }
                if stream.write_all(&buf[..n]).await.is_err() {
                    break;
                }
            }
        });

        Ok(response)
    }

    async fn connect_secure(
        &self,
        key: String,
        upgrade: xidl_rust_axum::upgrade::Upgrade,
    ) -> xidl_rust_axum::Result<xidl_rust_axum::axum::response::Response> {
        if key != "secret" {
            return Err(xidl_rust_axum::Error::new(403, "forbidden"));
        }

        let response = upgrade.on_upgrade(|mut stream| async move {
            let _ = stream.write_all(b"secure session").await;
        });

        Ok(response)
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_http_upgrade_success_and_early_rejection() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let addr = listener.local_addr().expect("read local addr");
    listener
        .set_nonblocking(true)
        .expect("set listener nonblocking");
    let listener = tokio::net::TcpListener::from_std(listener).expect("adopt listener for tokio");

    let server = HttpUpgradeServiceServer::new(ImUpgradeServer);
    let task = tokio::spawn(async move {
        xidl_rust_axum::Server::builder()
            .with_service(server)
            .serve_with_listener(listener)
            .await
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let http = xidl_rust_axum::reqwest::Client::builder()
        .build()
        .expect("build reqwest client");
    let client = HttpUpgradeServiceClient::with_http(format!("http://{}", addr), http);

    // Test 1: Successful Upgrade (raw)
    let mut stream = client
        .connect_raw("valid".to_string())
        .await
        .expect("upgrade connection successfully");

    stream.write_all(b"world").await.unwrap();
    stream.flush().await.unwrap();

    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf).await.unwrap();
    assert_eq!(&buf[..n], b"hello: world");

    // Test 2: Successful Upgrade (echo)
    let mut stream = client
        .connect_echo()
        .await
        .expect("upgrade connection successfully");

    stream.write_all(b"echo me").await.unwrap();
    stream.flush().await.unwrap();

    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf).await.unwrap();
    assert_eq!(&buf[..n], b"echo me");

    // Test 3: Successful Upgrade (secure)
    let mut stream = client
        .connect_secure("secret".to_string())
        .await
        .expect("upgrade connection successfully");

    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf).await.unwrap();
    assert_eq!(&buf[..n], b"secure session");

    // Test 4: Header Rejection
    let result = client.connect_secure("wrong".to_string()).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, 403);

    // Test 5: Early Rejection
    let result = client.connect_raw("invalid".to_string()).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, 401);

    task.abort();
}
