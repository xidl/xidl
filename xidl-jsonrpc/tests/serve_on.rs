use tokio::io::{BufReader, split};
#[cfg(feature = "tokio-net")]
use tokio::net::TcpStream;
use xidl_jsonrpc::Client;
use xidl_jsonrpc::Error;
use xidl_jsonrpc::Handler;

struct EchoHandler;

#[async_trait::async_trait]
impl Handler for EchoHandler {
    async fn handle(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, Error> {
        if method == "echo" {
            Ok(params)
        } else {
            Err(Error::method_not_found(method))
        }
    }
}

async fn call_echo_over_stream<S>(stream: S) -> Result<String, Error>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let (reader, writer) = split(stream);
    let mut client = Client::new(BufReader::new(reader), writer);
    client
        .call::<_, String>("echo", serde_json::json!("ok"))
        .await
}

async fn connect_with_retry(
    endpoint: &str,
) -> std::io::Result<Box<dyn xidl_jsonrpc::transport::Stream + Unpin + Send + 'static>> {
    let mut last_err = None;
    for _ in 0..50 {
        match xidl_jsonrpc::transport::connect(endpoint).await {
            Ok(stream) => return Ok(stream),
            Err(err) => {
                last_err = Some(err);
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        }
    }
    Err(last_err.unwrap_or_else(|| {
        std::io::Error::other(format!("failed to connect endpoint: {endpoint}"))
    }))
}

fn random_endpoint(prefix: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before epoch")
        .as_nanos();
    format!("{prefix}-{nanos}")
}

#[cfg(all(feature = "tokio-net", unix))]
fn random_ipc_uri(prefix: &str) -> String {
    let path =
        std::path::Path::new("/tmp").join(format!("xj-{prefix}-{}.sock", random_endpoint("ipc")));
    format!("ipc://{}", path.display())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn serve_on_inproc_uri() {
    let endpoint = random_endpoint("serve-on-inproc");
    let uri = format!("inproc://{endpoint}");
    let task = tokio::spawn(async move {
        xidl_jsonrpc::Server::builder()
            .with_service(EchoHandler)
            .serve_on(&uri)
            .await
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let stream = xidl_jsonrpc::transport::connect_inproc(&endpoint).expect("connect inproc");
    let result = call_echo_over_stream(stream).await.expect("rpc call");
    assert_eq!(result, "ok");

    task.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[cfg(all(feature = "tokio-net", unix))]
async fn serve_on_ipc_uri() {
    let uri = random_ipc_uri("serve-on-ipc");
    let connect_uri = uri.clone();
    let task = tokio::spawn(async move {
        xidl_jsonrpc::Server::builder()
            .with_service(EchoHandler)
            .serve_on(&uri)
            .await
    });

    let stream = match connect_with_retry(&connect_uri).await {
        Ok(stream) => stream,
        Err(err) => {
            if task.is_finished() {
                let result = task.await.expect("join ipc server task");
                if matches!(
                    &result,
                    Err(Error::Io(io_err)) if io_err.kind() == std::io::ErrorKind::PermissionDenied
                ) {
                    return;
                }
                panic!("connect ipc failed: {err}; server result: {result:?}");
            }
            panic!("connect ipc: {err}");
        }
    };
    let result = call_echo_over_stream(stream).await.expect("rpc call");
    assert_eq!(result, "ok");

    task.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[cfg(all(feature = "tokio-net", windows))]
async fn ipc_uri_is_unsupported() {
    let endpoint = "ipc://xidl-jsonrpc-unsupported";
    let err = xidl_jsonrpc::Server::builder()
        .with_service(EchoHandler)
        .serve_on(endpoint)
        .await
        .expect_err("ipc should be unsupported");
    assert!(
        err.to_string()
            .contains("ipc transport is unsupported on windows")
    );

    let err = match xidl_jsonrpc::transport::connect(endpoint).await {
        Ok(_) => panic!("ipc connect should be unsupported"),
        Err(err) => err,
    };
    assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn inproc_connect_before_bind() {
    let endpoint = random_endpoint("connect-before-bind");
    let stream = xidl_jsonrpc::transport::connect_inproc(&endpoint).expect("connect inproc");

    let call_task = tokio::spawn(async move { call_echo_over_stream(stream).await });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let uri = format!("inproc://{endpoint}");
    let server_task = tokio::spawn(async move {
        xidl_jsonrpc::Server::builder()
            .with_service(EchoHandler)
            .serve_on(&uri)
            .await
    });

    let result = tokio::time::timeout(std::time::Duration::from_secs(2), call_task)
        .await
        .expect("rpc call timed out")
        .expect("join call task")
        .expect("rpc call failed");
    assert_eq!(result, "ok");

    server_task.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[cfg(feature = "tokio-net")]
async fn serve_on_tcp_uri() {
    let probe = match std::net::TcpListener::bind("127.0.0.1:0") {
        Ok(probe) => probe,
        Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => return,
        Err(err) => panic!("bind probe: {err}"),
    };
    let addr = probe.local_addr().expect("probe addr");
    drop(probe);

    let uri = format!("tcp://{addr}");
    let task = tokio::spawn(async move {
        xidl_jsonrpc::Server::builder()
            .with_service(EchoHandler)
            .serve_on(&uri)
            .await
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let stream = TcpStream::connect(addr).await.expect("connect tcp");
    let result = call_echo_over_stream(stream).await.expect("rpc call");
    assert_eq!(result, "ok");

    task.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[cfg(feature = "tokio-websocket")]
async fn serve_on_websocket_uri() {
    let probe = std::net::TcpListener::bind("127.0.0.1:0").expect("bind probe");
    let addr = probe.local_addr().expect("probe addr");
    drop(probe);

    let uri = format!("ws://{addr}/rpc");
    let serve_uri = uri.clone();
    let task = tokio::spawn(async move {
        xidl_jsonrpc::Server::builder()
            .with_service(EchoHandler)
            .serve_on(&serve_uri)
            .await
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let stream = xidl_jsonrpc::transport::connect(&uri)
        .await
        .expect("connect websocket");
    let result = call_echo_over_stream(stream).await.expect("rpc call");
    assert_eq!(result, "ok");

    task.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[cfg(feature = "tokio-websocket")]
async fn wss_requires_cert_and_key() {
    let endpoint = "wss://127.0.0.1:18443/rpc";
    let err = xidl_jsonrpc::Server::builder()
        .with_service(EchoHandler)
        .serve_on(endpoint)
        .await
        .expect_err("wss should require cert and key");
    assert!(err.to_string().contains("missing tls parameter `cert`"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[cfg(feature = "tokio-tls")]
async fn tls_requires_cert_and_key() {
    let endpoint = "tls://127.0.0.1:19443";
    let err = xidl_jsonrpc::Server::builder()
        .with_service(EchoHandler)
        .serve_on(endpoint)
        .await
        .expect_err("tls should require cert and key");
    assert!(err.to_string().contains("missing tls parameter `cert`"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[cfg(feature = "tokio-websocket")]
async fn wss_connect_requires_ca() {
    let endpoint = "wss://127.0.0.1:18443/rpc";
    let err = match xidl_jsonrpc::transport::connect(endpoint).await {
        Ok(_) => panic!("wss client should require ca"),
        Err(err) => err,
    };
    assert!(err.to_string().contains("missing tls parameter `ca`"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[cfg(feature = "tokio-tls")]
async fn tls_connect_requires_ca() {
    let endpoint = "tls://127.0.0.1:19443";
    let err = match xidl_jsonrpc::transport::connect(endpoint).await {
        Ok(_) => panic!("tls client should require ca"),
        Err(err) => err,
    };
    assert!(err.to_string().contains("missing tls parameter `ca`"));
}
