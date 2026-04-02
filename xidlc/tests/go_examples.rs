use std::path::Path;
use std::process::Command;

#[test]
fn go_examples_pass() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let examples_dir = manifest_dir.join("../golang/xidlc-examples");
    let go_cache = std::env::temp_dir().join("xidl-go-cache");
    let go_path = std::env::temp_dir().join("xidl-go-path");

    let status = Command::new("go")
        .arg("test")
        .arg("./...")
        .current_dir(&examples_dir)
        .env("GOCACHE", &go_cache)
        .env("GOPATH", &go_path)
        .status()
        .expect("failed to run go test");

    assert!(status.success(), "go examples test suite failed");
}
