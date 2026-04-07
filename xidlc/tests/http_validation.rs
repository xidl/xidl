use std::collections::HashMap;
use std::fs;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

fn test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn invalid_case_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("invalid")
        .join(name)
}

fn invalid_case_source(name: &str) -> (PathBuf, String) {
    let path = invalid_case_path(name);
    let source = fs::read_to_string(&path).expect("read invalid idl fixture");
    (path, source)
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&'static str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "unknown panic payload".to_string()
    }
}

fn generate_error(lang: &str, fixture: &str) -> String {
    let _guard = test_lock().lock().expect("lock validation tests");
    let (_path, source) = invalid_case_source(fixture);
    match panic::catch_unwind(AssertUnwindSafe(|| {
        xidlc::generate_from_source(lang, &source, HashMap::new())
    })) {
        Ok(Ok(_)) => panic!("fixture should fail"),
        Ok(Err(err)) => err.to_string(),
        Err(payload) => panic_message(payload),
    }
}

#[test]
fn rejects_invalid_stream_codecs_for_http_targets() {
    for lang in ["axum", "ts", "go-http", "python-http"] {
        let err = generate_error(lang, "http_stream_invalid_codec.idl");
        assert!(
            err.contains("unsupported @stream_codec value"),
            "{lang}: {err}"
        );
    }
}

#[test]
fn rejects_invalid_server_stream_methods_for_http_targets() {
    for lang in ["axum", "ts", "go-http", "python-http"] {
        let err = generate_error(lang, "http_stream_invalid_server_method.idl");
        assert!(err.contains("@server_stream method"), "{lang}: {err}");
        assert!(err.contains("must use GET"), "{lang}: {err}");
    }
}

#[test]
fn rejects_typescript_bidi_stream_fixture() {
    let err = generate_error("ts", "http_stream_bidi_typescript.idl");
    assert!(err.contains("does not support @bidi_stream"), "{err}");
}

#[test]
fn rejects_non_body_client_stream_inputs_for_axum_and_typescript() {
    for lang in ["axum", "ts", "go-http", "python-http"] {
        let err = generate_error(lang, "http_client_stream_path_param.idl");
        assert!(
            err.contains("body parameters only") || err.contains("@client_stream"),
            "{lang}: {err}"
        );
    }
}

#[test]
fn rejects_duplicate_security_annotations() {
    for lang in ["axum", "go-http", "python-http"] {
        let err = generate_error(lang, "http_security_duplicate_basic.idl");
        assert!(
            err.contains("duplicate @http_basic annotation"),
            "{lang}: {err}"
        );
    }
}

#[test]
fn rejects_conflicting_no_security_annotations() {
    for lang in ["axum", "go-http", "python-http"] {
        let err = generate_error(lang, "http_security_conflict_no_security.idl");
        assert!(
            err.contains("@no_security cannot be combined with other security annotations"),
            "{lang}: {err}"
        );
    }
}

#[test]
fn rejects_unary_conflicting_param_sources() {
    for lang in ["axum", "ts", "go-http"] {
        let err = generate_error(lang, "http_unary_conflicting_param_source.idl");
        assert!(
            err.contains("conflicting source annotations"),
            "{lang}: {err}"
        );
    }
}

#[test]
fn rejects_missing_query_template_bindings() {
    for lang in ["axum", "ts", "go-http"] {
        let err = generate_error(lang, "http_unary_query_template_missing_binding.idl");
        assert!(
            err.contains(
                "query template variable 'region' has no matching request-side query parameter"
            ),
            "{lang}: {err}"
        );
    }
}

#[test]
fn rejects_duplicate_route_bindings_for_axum() {
    for lang in ["axum", "go-http"] {
        let err = generate_error(lang, "http_unary_duplicate_route_binding.idl");
        assert!(
            err.contains("duplicate HTTP route binding"),
            "{lang}: {err}"
        );
    }
}

#[test]
fn rejects_additional_invalid_security_annotations() {
    for lang in ["axum", "go-http", "python-http"] {
        let duplicate_bearer = generate_error(lang, "http_security_duplicate_bearer.idl");
        assert!(
            duplicate_bearer.contains("duplicate @http_bearer annotation"),
            "{lang}: {duplicate_bearer}"
        );

        let missing_name = generate_error(lang, "http_security_api_key_missing_name.idl");
        assert!(
            missing_name.contains("@api_key requires non-empty name=..."),
            "{lang}: {missing_name}"
        );

        let invalid_location = generate_error(lang, "http_security_api_key_invalid_location.idl");
        assert!(
            invalid_location.contains("must be one of header|query|cookie"),
            "{lang}: {invalid_location}"
        );
    }
}

#[test]
fn rejects_additional_invalid_stream_shapes() {
    for lang in ["axum", "ts", "go-http", "python-http"] {
        let err = generate_error(lang, "http_stream_mutually_exclusive.idl");
        assert!(err.contains("mutually exclusive"), "{lang}: {err}");
    }

    for lang in ["axum", "ts", "go-http", "python-http"] {
        let err = generate_error(lang, "http_stream_client_sse.idl");
        assert!(
            err.contains("supports only NDJSON for @client_stream methods")
                || err.contains("@stream_codec(\"sse\") requires @server_stream"),
            "{lang}: {err}"
        );
    }
}
