use super::*;
use crate::hir::{
    Annotation, AnnotationParam, AnnotationParams, BinaryOperator, ConstExpr, Literal,
};

fn builtin(name: &str, params: Option<AnnotationParams>) -> Annotation {
    Annotation::Builtin {
        name: name.to_string(),
        params,
    }
}

fn string(value: &str) -> ConstExpr {
    ConstExpr::Literal(Literal::StringLiteral(format!("\"{value}\"")))
}

fn raw_string(value: &str) -> ConstExpr {
    ConstExpr::Literal(Literal::StringLiteral(value.to_string()))
}

#[test]
fn effective_cors_inherits_and_overrides() {
    let inherited = effective_cors(&[builtin("cors", None)], &[])
        .expect("cors")
        .expect("present");
    assert_eq!(inherited, HttpCorsProfile::Any);

    let override_value = effective_cors(
        &[builtin("cors", None)],
        &[builtin(
            "cors",
            Some(AnnotationParams::ConstExpr(ConstExpr::BinaryExpr(
                BinaryOperator::Or,
                Box::new(string("https://console.example.com")),
                Box::new(string("https://admin.example.com")),
            ))),
        )],
    )
    .expect("cors")
    .expect("present");
    assert_eq!(
        override_value,
        HttpCorsProfile::Origins(vec![
            "https://console.example.com".to_string(),
            "https://admin.example.com".to_string(),
        ])
    );
}

#[test]
fn collect_cors_rejects_duplicates_and_named_params() {
    let err = collect_cors(&[builtin("cors", None), builtin("cors", None)]).expect_err("duplicate");
    assert!(err.contains("duplicate @cors"));

    let err = collect_cors(&[builtin(
        "cors",
        Some(AnnotationParams::Raw(
            r#"origin="https://app.example.com""#.to_string(),
        )),
    )])
    .expect_err("named params");
    assert!(err.contains("only accepts string literals joined by '|'"));

    let err = collect_cors(&[builtin(
        "cors",
        Some(AnnotationParams::Raw(
            r#""https://app.example.com", "https://admin.example.com""#.to_string(),
        )),
    )])
    .expect_err("raw list");
    assert!(err.contains("only accepts string literals joined by '|'"));

    let err = collect_cors(&[builtin(
        "cors",
        Some(AnnotationParams::Params(vec![AnnotationParam {
            ident: "origin".to_string(),
            value: Some(string("https://app.example.com")),
        }])),
    )])
    .expect_err("params");
    assert!(err.contains("only accepts string literals joined by '|'"));
}

#[test]
fn collect_cors_rejects_invalid_const_expr_forms() {
    let err = collect_cors(&[builtin(
        "cors",
        Some(AnnotationParams::ConstExpr(ConstExpr::BinaryExpr(
            BinaryOperator::Add,
            Box::new(string("https://app.example.com")),
            Box::new(string("https://admin.example.com")),
        ))),
    )])
    .expect_err("operator");
    assert!(err.contains("only accepts string literals joined by '|'"));

    let err = collect_cors(&[builtin(
        "cors",
        Some(AnnotationParams::ConstExpr(raw_string(
            "https://app.example.com",
        ))),
    )])
    .expect_err("unquoted");
    assert!(err.contains("only accepts string literals joined by '|'"));
}

#[test]
fn collect_cors_rejects_empty_and_invalid_origins() {
    let err = collect_cors(&[builtin(
        "cors",
        Some(AnnotationParams::ConstExpr(string(""))),
    )])
    .expect_err("empty");
    assert!(err.contains("must not be empty"));

    let err = collect_cors(&[builtin(
        "cors",
        Some(AnnotationParams::ConstExpr(string(
            "https://例子.example.com",
        ))),
    )])
    .expect_err("non-ascii");
    assert!(err.contains("invalid @cors origin"));
}

#[test]
fn collect_cors_accepts_wildcard_origin() {
    let profile = collect_cors(&[builtin(
        "cors",
        Some(AnnotationParams::ConstExpr(string("*"))),
    )])
    .expect("cors")
    .expect("present");
    assert_eq!(profile, HttpCorsProfile::Origins(vec!["*".to_string()]));
}
