//! End-to-end tests of the bundled in-process compiler backend.
#![cfg(feature = "bundle")]

use std::fs;
use std::path::PathBuf;
use xidl_build::Error;

const HELLO_WORLD_IDL: &str = include_str!("fixtures/hello_world.idl");

fn scratch_dir(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!("xidl-build-bundle-{}-{label}", std::process::id()))
}

#[test]
fn bundle_backend_generates_openapi_artifact() {
    let scratch = scratch_dir("openapi");
    let out_dir = scratch.join("out");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let input = scratch.join("hello_world.idl");
    fs::write(&input, HELLO_WORLD_IDL).expect("failed to write input idl");

    xidl_build::Builder::new()
        .with_lang("openapi")
        .with_out_dir(&out_dir)
        .compile(&[&input])
        .expect("bundled openapi generation should succeed");

    assert!(
        out_dir.join("openapi_hello_world.json").exists(),
        "openapi artifact missing in {}",
        out_dir.display()
    );
    let _ = fs::remove_dir_all(&scratch);
}

#[test]
fn bundle_backend_reports_compile_diagnostics() {
    let scratch = scratch_dir("diagnostics");
    let out_dir = scratch.join("out");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let input = scratch.join("broken.idl");
    fs::write(&input, "interface Broken {{").expect("failed to write input idl");

    let failure = xidl_build::Builder::new()
        .with_lang("openapi")
        .with_out_dir(&out_dir)
        .compile(&[&input])
        .expect_err("malformed IDL must fail bundled generation");
    match failure {
        Error::Compile(message) => {
            assert!(
                message.contains("broken.idl"),
                "unexpected message: {message}"
            );
        }
        other => panic!("expected Error::Compile, got {other:?}"),
    }
    let _ = fs::remove_dir_all(&scratch);
}
