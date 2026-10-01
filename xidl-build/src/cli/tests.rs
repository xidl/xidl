//! Unit tests for CLI backend command construction and request validation.

use super::CliBackend;
use crate::builder::GenerateSpec;
use crate::error::Error;
use std::path::PathBuf;
use std::process::Command;

fn spec(lang: &str, client: bool, server: bool, mock: bool) -> GenerateSpec {
    GenerateSpec {
        lang: lang.to_string(),
        out_dir: PathBuf::from("/tmp/xidl-build-test-out"),
        client,
        server,
        mock,
        inputs: vec![PathBuf::from("api/city.idl")],
    }
}

fn argv_of(command: &Command) -> Vec<String> {
    command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect()
}

fn assert_config_error(command: Result<Command, Error>, expected: &str) {
    match command.expect_err("request must be rejected before spawning xidlc") {
        Error::Config(message) => {
            assert!(message.contains(expected), "unexpected message: {message}");
        }
        other => panic!("expected Error::Config, got {other:?}"),
    }
}

#[test]
fn plain_generator_invocation_passes_no_flags() {
    let command = CliBackend::build_command(&spec("openapi", true, true, false))
        .expect("openapi request must build");
    assert_eq!(command.get_program().to_string_lossy(), "xidlc");
    assert_eq!(
        argv_of(&command),
        vec![
            "gen",
            "--out-dir",
            "/tmp/xidl-build-test-out",
            "openapi",
            "api/city.idl"
        ]
    );
}

#[test]
fn client_server_generator_passes_requested_flags() {
    let command = CliBackend::build_command(&spec("rust-axum", true, false, true))
        .expect("rust-axum request must build");
    assert_eq!(
        argv_of(&command),
        vec![
            "gen",
            "--out-dir",
            "/tmp/xidl-build-test-out",
            "rust-axum",
            "--client",
            "--mock",
            "api/city.idl"
        ]
    );
}

#[test]
fn client_server_generator_defaults_to_both_sides() {
    let command = CliBackend::build_command(&spec("rust-jsonrpc", true, true, false))
        .expect("rust-jsonrpc request must build");
    let argv = argv_of(&command);
    assert!(argv.contains(&"--client".to_string()));
    assert!(argv.contains(&"--server".to_string()));
    assert!(!argv.contains(&"--mock".to_string()));
}

#[test]
fn rejects_client_and_server_disabled() {
    assert_config_error(
        CliBackend::build_command(&spec("rust", false, false, false)),
        "client=false",
    );
}

#[test]
fn rejects_mock_for_plain_generator() {
    assert_config_error(
        CliBackend::build_command(&spec("openapi", true, true, true)),
        "--mock",
    );
}

#[test]
#[cfg(unix)]
fn failure_message_prefers_stderr_over_stdout() {
    let mut failing = Command::new("sh");
    failing
        .arg("-c")
        .arg("echo 'notice'; echo 'boom' >&2; exit 1");
    let output = failing.output().expect("failed to run helper shell");

    let message = CliBackend::failure_message(&output);
    assert!(message.contains("boom"), "unexpected message: {message}");
    assert!(!message.contains("notice"), "unexpected message: {message}");
}
