use super::super::*;
use std::collections::HashMap;
use std::panic::{self, AssertUnwindSafe};
use xidl_parser::hir;

fn parse_spec(source: &str) -> hir::Specification {
    let typed = xidl_parser::parser::parser_text(source).expect("parse typed ast");
    hir::Specification::from_typed_ast_with_properties(typed, HashMap::new())
}

fn render_openapi_json_from_spec(
    spec: &hir::Specification,
) -> Result<serde_json::Value, serde_json::Error> {
    let rest_hir = xidl_parser::rest_hir::project(spec).expect("project http hir");
    render_openapi_json(spec, &rest_hir)
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

#[test]
fn render_openapi_json_rejects_invalid_stream_codec() {
    let spec = parse_spec(
        r#"
        interface StreamApi {
          @server_stream
          @stream_codec("yaml")
          string watch();
        };
        "#,
    );
    let payload = panic::catch_unwind(AssertUnwindSafe(|| render_openapi_json_from_spec(&spec)))
        .expect_err("invalid stream codec should panic");
    let message = panic_message(payload);
    assert!(message.contains("unsupported @stream_codec value"));
}

#[test]
fn render_openapi_json_rejects_invalid_stream_method() {
    let spec = parse_spec(
        r#"
        interface StreamApi {
          @server_stream
          @stream_codec("sse")
          @post(path = "/watch")
          string watch();
        };
        "#,
    );
    let payload = panic::catch_unwind(AssertUnwindSafe(|| render_openapi_json_from_spec(&spec)))
        .expect_err("invalid stream method should panic");
    let message = panic_message(payload);
    assert!(message.contains("@server_stream method 'watch' must use GET"));
}

#[test]
fn render_openapi_json_rejects_duplicate_security_annotations() {
    let spec = parse_spec(
        r#"
        interface SecurityApi {
          @http_basic
          @http_basic
          @get(path = "/reports")
          string list_reports();
        };
        "#,
    );
    let payload = panic::catch_unwind(AssertUnwindSafe(|| render_openapi_json_from_spec(&spec)))
        .expect_err("duplicate security annotations should panic");
    let message = panic_message(payload);
    assert!(message.contains("duplicate @http_basic annotation"));
}

#[test]
fn render_openapi_json_rejects_conflicting_no_security_annotations() {
    let spec = parse_spec(
        r#"
        interface SecurityApi {
          @no_security
          @http_bearer
          @get(path = "/reports")
          string list_reports();
        };
        "#,
    );
    let payload = panic::catch_unwind(AssertUnwindSafe(|| render_openapi_json_from_spec(&spec)))
        .expect_err("conflicting security annotations should panic");
    let message = panic_message(payload);
    assert!(message.contains("@no_security cannot be combined with other security annotations"));
}

#[test]
fn render_openapi_json_rejects_conflicting_param_sources() {
    let spec = parse_spec(
        r#"
        interface HttpApi {
          @get(path = "/users/{id}")
          string get_user(
            @path("id") @query("user_id") string id
          );
        };
        "#,
    );
    let payload = panic::catch_unwind(AssertUnwindSafe(|| render_openapi_json_from_spec(&spec)))
        .expect_err("conflicting parameter sources should panic");
    let message = panic_message(payload);
    assert!(message.contains("conflicting source annotations"));
}

#[test]
fn render_openapi_json_rejects_missing_query_template_binding() {
    let spec = parse_spec(
        r#"
        interface HttpApi {
          @get(path = "/users/{id}{?lang,region}")
          string get_user(
            @path("id") string id,
            @query("lang") string lang
          );
        };
        "#,
    );
    let payload = panic::catch_unwind(AssertUnwindSafe(|| render_openapi_json_from_spec(&spec)))
        .expect_err("missing query template binding should panic");
    let message = panic_message(payload);
    assert!(
        message.contains(
            "query template variable 'region' has no matching request-side query parameter"
        )
    );
}

#[test]
fn render_openapi_json_rejects_duplicate_route_bindings() {
    let spec = parse_spec(
        r#"
        interface HttpApi {
          @get(path = "/users/{id}")
          string get_user(@path("id") string id);

          @get(path = "/users/{id}")
          string fetch_user(@path("id") string id);
        };
        "#,
    );
    let payload = panic::catch_unwind(AssertUnwindSafe(|| render_openapi_json_from_spec(&spec)))
        .expect_err("duplicate route binding should panic");
    let message = panic_message(payload);
    assert!(message.contains("duplicate HTTP route binding"));
}

#[test]
fn render_openapi_json_rejects_additional_invalid_security_annotations() {
    let duplicate_bearer = parse_spec(
        r#"
        interface SecurityApi {
          @http_bearer
          @http_bearer
          @get(path = "/reports")
          string list_reports();
        };
        "#,
    );
    let payload = panic::catch_unwind(AssertUnwindSafe(|| {
        render_openapi_json_from_spec(&duplicate_bearer)
    }))
    .expect_err("duplicate bearer should panic");
    let message = panic_message(payload);
    assert!(message.contains("duplicate @http_bearer annotation"));

    let missing_name = parse_spec(
        r#"
        interface SecurityApi {
          @api_key(in = "header")
          @get(path = "/reports")
          string list_reports();
        };
        "#,
    );
    let payload = panic::catch_unwind(AssertUnwindSafe(|| {
        render_openapi_json_from_spec(&missing_name)
    }))
    .expect_err("api key missing name should panic");
    let message = panic_message(payload);
    assert!(message.contains("@api_key requires non-empty name=..."));

    let invalid_location = parse_spec(
        r#"
        interface SecurityApi {
          @api_key(in = "body", name = "auth")
          @get(path = "/reports")
          string list_reports();
        };
        "#,
    );
    let payload = panic::catch_unwind(AssertUnwindSafe(|| {
        render_openapi_json_from_spec(&invalid_location)
    }))
    .expect_err("api key invalid location should panic");
    let message = panic_message(payload);
    assert!(message.contains("must be one of header|query|cookie"));
}

#[test]
fn render_openapi_json_rejects_additional_invalid_stream_shapes() {
    let mutually_exclusive = parse_spec(
        r#"
        interface StreamApi {
          @server_stream
          @client_stream
          @stream_codec("ndjson")
          @post(path = "/events")
          string exchange(string payload);
        };
        "#,
    );
    let payload = panic::catch_unwind(AssertUnwindSafe(|| {
        render_openapi_json_from_spec(&mutually_exclusive)
    }))
    .expect_err("mutually exclusive stream annotations should panic");
    let message = panic_message(payload);
    assert!(message.contains("mutually exclusive"));

    let client_sse = parse_spec(
        r#"
        interface StreamApi {
          @client_stream
          @stream_codec("sse")
          @post(path = "/upload")
          string upload(sequence<octet> chunk);
        };
        "#,
    );
    let payload = panic::catch_unwind(AssertUnwindSafe(|| {
        render_openapi_json_from_spec(&client_sse)
    }))
    .expect_err("client stream sse should panic");
    let message = panic_message(payload);
    assert!(
        message.contains("supports only NDJSON for @client_stream methods")
            || message.contains("requires @server_stream")
    );
}
