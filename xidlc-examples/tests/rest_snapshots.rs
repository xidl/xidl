use std::path::Path;
use std::process::Command;

use xidlc_examples::city_rest::{SmartCityRestApiServer, SmartCityRestService};
use xidlc_examples::e2e_test::{
    E2eHttpRouteAndBodyServer, E2eHttpSecurityServer, E2ePathSeverServer, E2eTypeServerServer,
    MockE2eHttpRouteAndBody, MockE2eHttpSecurity, MockE2ePathSever, MockE2eTypeServer,
};
use xidlc_examples::rest_media_types::{RestMediaTypesApiServer, RestMediaTypesService};
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
            .with_service(SmartCityRestApiServer::new(SmartCityRestService))
            .with_service(RestMediaTypesApiServer::new(RestMediaTypesService))
            .with_service(E2ePathSeverServer::new(MockE2ePathSever))
            .with_service(E2eHttpRouteAndBodyServer::new(MockE2eHttpRouteAndBody))
            .with_service(E2eHttpSecurityServer::new(MockE2eHttpSecurity))
            .with_service(E2eTypeServerServer::new(MockE2eTypeServer))
            .serve_with_listener(listener)
            .await
    });

    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    let defs_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/rest_snapshots/defs");
    let mut hurl_files: Vec<_> = std::fs::read_dir(defs_dir)
        .expect("read defs dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map_or(false, |ext| ext == "hurl"))
        .collect();
    hurl_files.sort();

    for hurl_file in hurl_files {
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
            eprintln!("Hurl file: {:?}", hurl_file);
            eprintln!("Hurl stdout:\n{}", String::from_utf8_lossy(&output.stdout));
            eprintln!("Hurl stderr:\n{}", String::from_utf8_lossy(&output.stderr));
            panic!("Hurl tests failed for {:?}", hurl_file);
        }
    }

    task.abort();
}
