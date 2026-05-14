use super::*;
use crate::hir::{Annotation, AnnotationParams, BinaryOperator, ConstExpr, Literal};

fn builtin(name: &str, raw: &str) -> Annotation {
    Annotation::Builtin {
        name: name.to_string(),
        params: Some(AnnotationParams::Raw(raw.to_string())),
    }
}

fn builtin_expr(name: &str, expr: ConstExpr) -> Annotation {
    Annotation::Builtin {
        name: name.to_string(),
        params: Some(AnnotationParams::ConstExpr(expr)),
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

    let value_alias = deprecated_info(&[builtin("deprecated", r#""2026-01-05T01:02:03Z""#)])
        .expect("deprecated value alias")
        .expect("present");
    assert_eq!(value_alias.since.as_deref(), Some("2026-01-05T01:02:03Z"));
    assert_eq!(value_alias.after, None);

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
        &[
            builtin_expr(
                "cors",
                ConstExpr::BinaryExpr(
                    BinaryOperator::Or,
                    Box::new(ConstExpr::Literal(Literal::StringLiteral(
                        "\"https://app.example.com\"".to_string(),
                    ))),
                    Box::new(ConstExpr::Literal(Literal::StringLiteral(
                        "\"https://admin.example.com\"".to_string(),
                    ))),
                ),
            ),
            builtin("Consume", r#""application/json""#),
            builtin("Produce", r#""text/plain""#),
        ],
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

    let err = validate_http_annotations(
        "operation 'cors'",
        &[builtin("cors", r#"origin="https://app.example.com""#)],
    )
    .expect_err("bad cors");
    assert!(err.contains("@cors only accepts string literals joined by '|'"));

    validate_http_annotations(
        "operation 'empty'",
        &[builtin("Produces", ""), Annotation::Final],
    )
    .expect("empty media annotation should be ignored");
}
