use std::collections::HashMap;
use std::fs;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};

fn invalid_case_source(name: &str) -> String {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("invalid")
        .join(name);
    fs::read_to_string(&path).expect("read invalid idl fixture")
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

fn generate_error(lang: &str, fixture: &str, props: HashMap<String, serde_json::Value>) -> String {
    let source = invalid_case_source(fixture);
    match panic::catch_unwind(AssertUnwindSafe(|| {
        xidlc::generate_from_source(lang, &source, props)
    })) {
        Ok(Ok(_)) => panic!("fixture {fixture} should fail"),
        Ok(Err(err)) => err.to_string(),
        Err(payload) => panic_message(payload),
    }
}

#[test]
fn rejects_unknown_version_references() {
    for lang in ["openapi", "axum", "go-rest", "ts-rest", "rust"] {
        let err = generate_error(lang, "versioning_unknown_reference.idl", HashMap::new());
        assert!(err.contains("references ApiVersions::v9"), "{lang}: {err}");
        assert!(err.contains("v1 (v1), v2 (v2)"), "{lang}: {err}");
    }
}

#[test]
fn rejects_references_to_removed_declarations() {
    for lang in ["openapi", "axum", "go-rest", "ts-rest", "rust"] {
        let err = generate_error(lang, "versioning_dangling_reference.idl", HashMap::new());
        assert!(err.contains("references Old"), "{lang}: {err}");
        assert!(err.contains("does not exist in v2"), "{lang}: {err}");
    }
}

#[test]
fn rejects_unimplemented_version_annotations() {
    let err = generate_error(
        "openapi",
        "versioning_unsupported_annotation.idl",
        HashMap::new(),
    );
    assert!(err.contains("@madeOptional"), "{err}");
    assert!(err.contains("is not supported yet"), "{err}");
}

#[test]
fn rejects_version_selection_on_unversioned_contracts() {
    let props = HashMap::from([(
        "api_version".to_string(),
        serde_json::Value::String("v1".to_string()),
    )]);
    let err = generate_error("openapi", "versioning_plain.idl", props);
    assert!(
        err.contains("requires a @version enum in the specification"),
        "{err}"
    );
}

#[test]
fn rejects_unknown_api_version_selection() {
    let props = HashMap::from([(
        "api_version".to_string(),
        serde_json::Value::String("9.9".to_string()),
    )]);
    let err = generate_error("openapi", "versioning_dangling_reference.idl", props);
    assert!(err.contains("version 9.9 is not declared"), "{err}");
}
