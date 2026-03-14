use super::*;
use xidl_parser::hir;

#[test]
fn test_parse_timestamp() {
    let timestamp = [
        //
        0,
        1000,
        i64::MAX,
        i64::MIN,
    ];

    for case in timestamp {
        let _ = format_timestamp_filter(case);
    }
}

#[test]
fn test_doc_lines_from_raw() {
    let annotations = vec![hir::Annotation::Builtin {
        name: "doc".to_string(),
        params: Some(hir::AnnotationParams::Raw("\"hello\\nworld\"".to_string())),
    }];
    let doc = doc_lines_from_annotations(&annotations);
    assert_eq!(doc, vec!["hello", "world"]);
}

#[test]
fn test_doc_lines_from_const_expr() {
    let expr = hir::ConstExpr(hir::OrExpr::XorExpr(hir::XorExpr::AndExpr(
        hir::AndExpr::ShiftExpr(hir::ShiftExpr::AddExpr(hir::AddExpr::MultExpr(
            hir::MultExpr::UnaryExpr(hir::UnaryExpr::PrimaryExpr(hir::PrimaryExpr::Literal(
                hir::Literal::StringLiteral("hi".to_string()),
            ))),
        ))),
    )));
    let annotations = vec![hir::Annotation::Builtin {
        name: "doc".to_string(),
        params: Some(hir::AnnotationParams::ConstExpr(expr)),
    }];
    let doc = doc_lines_from_annotations(&annotations);
    assert_eq!(doc, vec!["hi"]);
}

#[test]
fn test_deprecated_info_normalizes_dates() {
    let annotations = vec![hir::Annotation::Builtin {
        name: "deprecated".to_string(),
        params: Some(hir::AnnotationParams::Raw(
            "since = \"2026-03-13\", after = \"2026-03-14T12:00:00+08:00\"".to_string(),
        )),
    }];
    let deprecated = deprecated_info(&annotations).unwrap().unwrap();
    assert_eq!(deprecated.since.as_deref(), Some("2026-03-13T00:00:00Z"));
    assert_eq!(deprecated.after.as_deref(), Some("2026-03-14T04:00:00Z"));
}

#[test]
fn test_effective_security_parses_oauth_and_api_key() {
    let annotations = vec![
        hir::Annotation::Builtin {
            name: "api-key".to_string(),
            params: Some(hir::AnnotationParams::Raw(
                "in = \"header\", name = \"X-API-Key\"".to_string(),
            )),
        },
        hir::Annotation::Builtin {
            name: "oauth2".to_string(),
            params: Some(hir::AnnotationParams::Raw(
                "scopes = [\"read:users\", \"write:users\"]".to_string(),
            )),
        },
    ];
    let security = effective_security(&[], &annotations).unwrap().unwrap();
    assert_eq!(security.len(), 2);
    assert!(matches!(
        &security[0],
        HttpSecurityRequirement::ApiKey {
            location: HttpApiKeyLocation::Header,
            name
        } if name == "X-API-Key"
    ));
    assert!(matches!(
        &security[1],
        HttpSecurityRequirement::OAuth2 { scopes }
        if scopes == &vec!["read:users".to_string(), "write:users".to_string()]
    ));
}

#[test]
fn test_validate_http_annotations_rejects_invalid_security_mix() {
    let annotations = vec![
        hir::Annotation::Builtin {
            name: "no-security".to_string(),
            params: None,
        },
        hir::Annotation::Builtin {
            name: "http-basic".to_string(),
            params: None,
        },
    ];
    let err = validate_http_annotations("op foo", &annotations).unwrap_err();
    assert!(err.contains("no-security"));
}

#[test]
fn test_http_stream_config_parses_and_validates() {
    let annotations = vec![
        hir::Annotation::Builtin {
            name: "server-stream".to_string(),
            params: None,
        },
        hir::Annotation::Builtin {
            name: "stream-codec".to_string(),
            params: Some(hir::AnnotationParams::Raw("\"sse\"".to_string())),
        },
    ];
    let config = http_stream_config(&annotations).unwrap();
    assert_eq!(config.kind, Some(HttpStreamKind::Server));
    assert_eq!(config.codec, HttpStreamCodec::Sse);
}

#[test]
fn test_http_stream_target_rejects_unsupported_bidi() {
    let annotations = vec![hir::Annotation::Builtin {
        name: "bidi-stream".to_string(),
        params: None,
    }];
    let config = http_stream_config(&annotations).unwrap();
    let err = validate_http_stream_target(
        "chat",
        config,
        HttpStreamTargetSupport {
            target: "typescript",
            supports_bidi: false,
            server_codec: HttpStreamCodec::Sse,
            client_codec: HttpStreamCodec::Ndjson,
            server_method: "GET",
            client_method: "POST",
            bidi_method: "GET",
        },
    )
    .unwrap_err();
    assert!(err.contains("does not support @bidi-stream"));
}

#[test]
fn test_http_stream_method_rejects_invalid_server_method() {
    let err = validate_http_stream_method(
        "watch",
        Some(HttpStreamKind::Server),
        "POST",
        HttpStreamTargetSupport {
            target: "openapi",
            supports_bidi: false,
            server_codec: HttpStreamCodec::Sse,
            client_codec: HttpStreamCodec::Ndjson,
            server_method: "GET",
            client_method: "POST",
            bidi_method: "GET",
        },
    )
    .unwrap_err();
    assert!(err.contains("must use GET"));
}
