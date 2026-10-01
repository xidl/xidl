//! CLI-mode request validation that never needs a `xidlc` executable.
#![cfg(not(feature = "bundle"))]

use xidl_build::Error;

#[test]
fn rejects_client_and_server_disabled() {
    let failure = xidl_build::Builder::new()
        .with_lang("rust")
        .with_client(false)
        .with_server(false)
        .with_out_dir("/tmp/xidl-build-config-errors")
        .compile(&["api/city.idl"])
        .expect_err("disabled client and server must fail in CLI mode");
    match failure {
        Error::Config(message) => {
            assert!(
                message.contains("client=false"),
                "unexpected message: {message}"
            );
        }
        other => panic!("expected Error::Config, got {other:?}"),
    }
}

#[test]
fn rejects_mock_for_plain_generator() {
    let failure = xidl_build::Builder::new()
        .with_lang("openapi")
        .with_mock(true)
        .with_out_dir("/tmp/xidl-build-config-errors")
        .compile(&["api/city.idl"])
        .expect_err("mock generation must fail for plain generators in CLI mode");
    match failure {
        Error::Config(message) => {
            assert!(message.contains("--mock"), "unexpected message: {message}");
        }
        other => panic!("expected Error::Config, got {other:?}"),
    }
}
