use std::path::Path;
use std::process::Command;

use xidlc_examples::rest_server::{RestServerServer, SimpleRestServer};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rest_snapshot_tests() {
    let listener = match std::net::TcpListener::bind("127.0.0.1:0") {
        Ok(listener) => listener,
        Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => return,
        Err(err) => panic!("bind ephemeral port: {err}"),
    };
    let addr = listener.local_addr().expect("read local addr");
    listener
        .set_nonblocking(true)
        .expect("set listener nonblocking");
    let listener = tokio::net::TcpListener::from_std(listener).expect("adopt listener for tokio");

    let task = tokio::spawn(async move {
        xidl_rust_axum::Server::builder()
            .with_service(RestServerServer::new(SimpleRestServer::new()))
            .serve_with_listener(listener)
            .await
    });

    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    let hurl_file =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/rest_snapshots/defs/rest_server.hurl");

    let output = Command::new("pnpm")
        .args([
            "exec",
            "hurl",
            "--test",
            "--variable",
            &format!("base_url=http://{}", addr),
            hurl_file.to_str().unwrap(),
        ])
        .output()
        .expect("failed to execute hurl");

    if !output.status.success() {
        eprintln!("Hurl stdout:\n{}", String::from_utf8_lossy(&output.stdout));
        eprintln!("Hurl stderr:\n{}", String::from_utf8_lossy(&output.stderr));
        panic!("Hurl tests failed");
    }

    task.abort();
}
