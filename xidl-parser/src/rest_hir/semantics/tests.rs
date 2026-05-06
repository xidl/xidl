use super::*;
use crate::hir::{Annotation, AnnotationParams};

fn builtin(name: &str, raw: &str) -> Annotation {
    Annotation::Builtin {
        name: name.to_string(),
        params: Some(AnnotationParams::Raw(raw.to_string())),
    }
}

#[test]
fn deprecated_info_normalizes_dates_and_validates_ranges() {
    assert_eq!(deprecated_info(&[]).unwrap(), None);

    let info = deprecated_info(&[builtin(
        "deprecated",
        r#"since="2026-01-02", after="2026-01-03""#,
    )])
    .expect("deprecated info")
    .expect("present");
    assert!(info.deprecated);
    assert!(info.since.is_some());
    assert!(info.after.is_some());

    let err = deprecated_info(&[builtin(
        "deprecated",
        r#"since="2026-01-04", after="2026-01-03""#,
    )])
    .expect_err("invalid range");
    assert!(err.contains("since <= after"));

    let err = deprecated_info(&[builtin("deprecated", r#"since="not-a-date""#)])
        .expect_err("invalid timestamp");
    assert!(err.contains("invalid @deprecated timestamp literal"));
}

#[test]
fn validate_http_annotations_checks_security_and_media_type() {
    validate_http_annotations(
        "operation 'ok'",
        &[builtin("Consumes", r#""application/json""#)],
    )
    .expect("valid annotation set");

    let err = validate_http_annotations(
        "operation 'bad'",
        &[builtin("Produces", r#""application/xml""#)],
    )
    .expect_err("bad media type");
    assert!(err.contains("unsupported @Produces(\"application/xml\") media type"));

    let err = validate_http_annotations(
        "operation 'secure'",
        &[builtin("no_security", ""), builtin("http_basic", "")],
    )
    .expect_err("bad security");
    assert!(err.contains("operation 'secure': @no_security cannot be combined"));
}
