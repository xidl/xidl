use super::super::*;
use std::collections::HashMap;
use xidl_parser::hir;

fn parse_spec(source: &str) -> hir::Specification {
    let typed = xidl_parser::parser::parser_text(source).expect("parse typed ast");
    hir::Specification::from(typed)
}

fn render_openapi_json_with_version(
    spec: &hir::Specification,
    version: OpenApiVersion,
) -> Result<serde_json::Value, crate::error::IdlcError> {
    let rest_hir = xidl_parser::rest_hir::project(spec).expect("project http hir");
    render_openapi_json(spec, &rest_hir, &Default::default(), version)
}

fn stream_spec() -> hir::Specification {
    parse_spec(
        r#"
        interface StreamApi {
          @server_stream
          @stream_codec("sse")
          string watch();

          @client_stream
          @stream_codec("ndjson")
          string upload(
            in string file_id,
            in sequence<octet> chunk
          );
        };
        "#,
    )
}

#[test]
fn forced_31_keeps_stream_schemas_without_32_fields() {
    let doc = render_openapi_json_with_version(&stream_spec(), OpenApiVersion::V31)
        .expect("render openapi json");
    assert_eq!(
        doc.get("openapi"),
        Some(&serde_json::Value::String("3.1.0".to_string()))
    );

    // Forced 3.1 is best-effort: the item schema stays in the standard slot and
    // no 3.2-only field or vendor extension is added.
    let server_content =
        &doc["paths"]["/watch"]["get"]["responses"]["200"]["content"]["text/event-stream"];
    assert!(server_content.get("schema").is_some());
    assert!(server_content.get("itemSchema").is_none());
    assert!(server_content.get("x-xidl-stream").is_none());

    let client_content =
        &doc["paths"]["/upload"]["post"]["requestBody"]["content"]["application/x-ndjson"];
    assert!(client_content.get("schema").is_some());
    assert!(client_content.get("itemSchema").is_none());
}

#[test]
fn openapi_version_from_props_reads_the_documented_values() {
    let props = |value: &str| {
        HashMap::from([(
            crate::driver::OPENAPI_VERSION_PROPERTY.to_string(),
            serde_json::Value::String(value.to_string()),
        )])
    };
    assert_eq!(
        OpenApiVersion::from_props(&HashMap::new()).expect("default"),
        OpenApiVersion::Auto
    );
    assert_eq!(
        OpenApiVersion::from_props(&props("auto")).expect("auto"),
        OpenApiVersion::Auto
    );
    assert_eq!(
        OpenApiVersion::from_props(&props("3.1")).expect("3.1"),
        OpenApiVersion::V31
    );
}

#[test]
fn openapi_version_from_props_rejects_unknown_values() {
    let props = HashMap::from([(
        crate::driver::OPENAPI_VERSION_PROPERTY.to_string(),
        serde_json::Value::String("3.0".to_string()),
    )]);
    let error = OpenApiVersion::from_props(&props).expect_err("unknown version");
    assert!(error.to_string().contains("--version"), "{error}");
    assert!(error.to_string().contains("3.0"), "{error}");
}
