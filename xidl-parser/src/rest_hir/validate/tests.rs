use super::*;
use crate::hir::{Annotation, AnnotationParams, ParamAttribute, TypeSpec};

fn builtin(name: &str, raw: &str) -> Annotation {
    Annotation::Builtin {
        name: name.to_string(),
        params: Some(AnnotationParams::Raw(raw.to_string())),
    }
}

fn param(name: &str, kind: HttpParamKind) -> HttpParam {
    HttpParam {
        name: name.to_string(),
        wire_name: name.to_string(),
        ty: TypeSpec::Boolean,
        kind,
        optional: false,
        flatten: false,
    }
}

#[test]
fn helpers_cover_default_sources_and_names() {
    assert_eq!(default_param_source(HttpMethod::Get), HttpParamKind::Query);
    assert_eq!(default_param_source(HttpMethod::Post), HttpParamKind::Body);
    assert_eq!(
        param_direction(Some(&ParamAttribute("out".to_string()))),
        HttpParamDirection::Out
    );
    assert_eq!(
        param_direction(Some(&ParamAttribute("inout".to_string()))),
        HttpParamDirection::InOut
    );
    assert_eq!(param_direction(None), HttpParamDirection::In);
    assert_eq!(http_method_name(HttpMethod::Patch), "PATCH");
}

#[test]
fn deprecated_and_basic_auth_resolution_prefers_item_annotations() {
    let interface = vec![
        builtin("deprecated", r#"since="2026-01-01""#),
        builtin("http_basic", r#"realm="api""#),
    ];
    let item = vec![
        builtin("deprecated", r#"since="2026-02-01""#),
        builtin("http_basic", r#"realm="item""#),
    ];
    assert_eq!(
        effective_basic_auth_realm(&interface, &item).as_deref(),
        Some("item")
    );
    assert_eq!(
        effective_deprecated(&interface, &item)
            .unwrap()
            .and_then(|info| info.since),
        deprecated_info(&item).unwrap().and_then(|info| info.since)
    );
}

#[test]
fn validates_header_cookie_stream_and_path_constraints() {
    assert!(
        validate_header_name("", "trace")
            .expect_err("empty header")
            .contains("empty @header")
    );
    assert!(validate_header_name("X-Trace-Id", "trace").is_ok());
    assert!(
        validate_header_name(":method", "trace")
            .expect_err("pseudo header")
            .contains("reserved pseudo-header")
    );
    assert!(
        validate_cookie_name("", "cookie")
            .expect_err("empty cookie")
            .contains("empty @cookie")
    );
    assert!(validate_cookie_name("session", "cookie").is_ok());
    assert!(
        validate_cookie_name("bad cookie", "cookie")
            .expect_err("cookie name")
            .contains("invalid @cookie")
    );

    let stream_err = validate_stream_shape(
        "upload",
        super::super::semantics::HttpStreamConfig {
            kind: Some(HttpStreamKind::Client),
            codec: HttpStreamCodec::Sse,
        },
    )
    .expect_err("stream shape");
    assert!(stream_err.contains("requires @server_stream"));

    let method_err =
        validate_stream_method("watch", Some(HttpStreamKind::Server), HttpMethod::Post)
            .expect_err("stream method");
    assert!(method_err.contains("must use GET"));

    let mut optional_path = param("id", HttpParamKind::Path);
    optional_path.optional = true;
    assert!(
        validate_projected_param(
            "get_city",
            &optional_path,
            HttpParamDirection::In,
            &[std::collections::HashSet::from(["id".to_string()])],
        )
        .expect_err("optional path")
        .contains("@optional")
    );

    let flatten_out = HttpParam {
        flatten: true,
        ..param("body", HttpParamKind::Body)
    };
    assert!(
        validate_projected_param("set", &flatten_out, HttpParamDirection::Out, &[])
            .expect_err("flatten out")
            .contains("@flatten")
    );
    assert!(
        validate_projected_param(
            "get_city",
            &param("id", HttpParamKind::Path),
            HttpParamDirection::In,
            &[
                std::collections::HashSet::from(["id".to_string()]),
                std::collections::HashSet::new(),
            ],
        )
        .expect_err("path in some routes")
        .contains("not present in every route template")
    );
    assert!(
        validate_projected_param(
            "get_city",
            &param("id", HttpParamKind::Header),
            HttpParamDirection::In,
            &[]
        )
        .is_ok()
    );
}

#[test]
fn validates_route_bindings_request_shape_and_head_constraints() {
    let routes = vec![HttpRoute {
        path: "/cities/{id}".to_string(),
        path_params: vec!["id".to_string()],
        query_params: vec!["region".to_string()],
    }];
    let path_counts = std::collections::HashMap::from([("id".to_string(), 1_usize)]);
    let query_counts = std::collections::HashMap::from([("region".to_string(), 1_usize)]);
    validate_route_bindings("get_city", &routes, &path_counts, &query_counts).unwrap();

    let err = validate_route_bindings(
        "get_city",
        &routes,
        &std::collections::HashMap::new(),
        &query_counts,
    )
    .expect_err("missing path binding");
    assert!(err.contains("has no matching request-side path parameter"));
    let err = validate_route_bindings(
        "get_city",
        &routes,
        &std::collections::HashMap::from([("id".to_string(), 2_usize)]),
        &query_counts,
    )
    .expect_err("duplicate path binding");
    assert!(err.contains("is bound by multiple"));

    let request_params = vec![
        param("a", HttpParamKind::Body),
        param("b", HttpParamKind::Header),
    ];
    let err = validate_request_shape("upload", Some(HttpStreamKind::Client), &request_params)
        .expect_err("non-body request");
    assert!(err.contains("body parameters only"));
    let err = validate_request_shape("chat", Some(HttpStreamKind::Bidi), &request_params)
        .expect_err("bidi non-body request");
    assert!(err.contains("@bidi_stream"));

    let flattened = vec![
        HttpParam {
            flatten: true,
            ..param("a", HttpParamKind::Body)
        },
        param("b", HttpParamKind::Body),
    ];
    let err = validate_request_shape("upload", None, &flattened).expect_err("flattened body");
    assert!(err.contains("requires exactly one request-side body parameter"));

    let head_err = validate_head_constraints(
        "head_city",
        HttpMethod::Head,
        &[param("etag", HttpParamKind::Header)],
        Some(&TypeSpec::Boolean),
    )
    .expect_err("head response");
    assert!(head_err.contains("must return void"));
    validate_head_constraints("head_city", HttpMethod::Head, &[], None).unwrap();
    assert_eq!(http_method_name(HttpMethod::Post), "POST");
    assert_eq!(http_method_name(HttpMethod::Put), "PUT");
    assert_eq!(http_method_name(HttpMethod::Delete), "DELETE");
    assert_eq!(http_method_name(HttpMethod::Head), "HEAD");
    assert_eq!(http_method_name(HttpMethod::Options), "OPTIONS");
}

#[test]
fn test_validate_upgrade_constraints() {
    // 1. Not an upgrade method
    assert!(validate_upgrade_constraints("op", false, None, HttpMethod::Get, &[], None).is_ok());

    // 2. Protocol not supplied
    let err =
        validate_upgrade_constraints("op", true, None, HttpMethod::Get, &[], None).unwrap_err();
    assert!(err.contains("requires a 'protocol' parameter"));

    // 3. Protocol is empty
    let err =
        validate_upgrade_constraints("op", true, Some(""), HttpMethod::Get, &[], None).unwrap_err();
    assert!(err.contains("cannot be empty"));

    // 4. Returns non-void
    let err = validate_upgrade_constraints(
        "op",
        true,
        Some("xidl-raw"),
        HttpMethod::Get,
        &[],
        Some(&TypeSpec::Boolean),
    )
    .unwrap_err();
    assert!(err.contains("must return void"));

    // 5. Has body parameter
    let request_params = vec![param("payload", HttpParamKind::Body)];
    let err = validate_upgrade_constraints(
        "op",
        true,
        Some("xidl-raw"),
        HttpMethod::Get,
        &request_params,
        None,
    )
    .unwrap_err();
    assert!(err.contains("cannot have @body parameters"));

    // 6. Uses non-GET method
    let err =
        validate_upgrade_constraints("op", true, Some("xidl-raw"), HttpMethod::Post, &[], None)
            .unwrap_err();
    assert!(err.contains("must use GET"));

    // 7. Successful validation
    assert!(
        validate_upgrade_constraints("op", true, Some("xidl-raw"), HttpMethod::Get, &[], None)
            .is_ok()
    );
}
