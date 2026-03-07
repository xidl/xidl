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

fn random_endpoint(prefix: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before epoch")
        .as_nanos();
    format!("{prefix}-{nanos}")
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
    let probe = std::net::TcpListener::bind("127.0.0.1:0").expect("bind probe");
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
