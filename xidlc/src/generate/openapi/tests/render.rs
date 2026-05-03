use super::super::*;
use crate::openapi::RefOr;
use crate::openapi::schema::Schema;
use std::collections::HashMap;
use xidl_parser::hir;

fn parse_spec(source: &str) -> hir::Specification {
    let typed = xidl_parser::parser::parser_text(source).expect("parse typed ast");
    hir::Specification::from_typed_ast_with_properties(typed, HashMap::new())
}

fn render_openapi_json_from_spec(
    spec: &hir::Specification,
) -> Result<serde_json::Value, serde_json::Error> {
    let http_hir = xidl_parser::http_hir::project(spec).expect("project http hir");
    render_openapi_json(spec, &http_hir)
}

fn doc_annotation(text: &str) -> hir::Annotation {
    hir::Annotation::Builtin {
        name: "doc".to_string(),
        params: Some(hir::AnnotationParams::Raw(format!("\"{}\"", text))),
    }
}

#[test]
fn schema_for_struct_applies_doc_to_fields() {
    let member = hir::Member {
        annotations: vec![doc_annotation("field doc")],
        ty: hir::TypeSpec::IntegerType(hir::IntegerType::I32),
        ident: vec![hir::Declarator::SimpleDeclarator(hir::SimpleDeclarator(
            "value".to_string(),
        ))],
        default: None,
        field_id: None,
    };
    let schema = schema::schema_for_struct(&[member]);
    let RefOr::T(Schema::Object(object)) = schema else {
        panic!("expected object schema");
    };
    let Some(prop) = object.properties.get("value") else {
        panic!("missing value property");
    };
    let RefOr::T(Schema::Object(prop_obj)) = prop else {
        panic!("expected object property schema");
    };
    assert_eq!(prop_obj.description.as_deref(), Some("field doc"));
}

#[test]
fn render_openapi_json_defaults_to_31_without_streams() {
    let spec = parse_spec(
        r#"
        interface HelloApi {
          string hello();
        };
        "#,
    );
    let doc = render_openapi_json_from_spec(&spec).expect("render openapi json");
    assert_eq!(
        doc.get("openapi"),
        Some(&serde_json::Value::String("3.1.0".to_string()))
    );
}

#[test]
fn render_openapi_json_uses_32_and_item_schema_for_streams() {
    let spec = parse_spec(
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
    );
    let doc = render_openapi_json_from_spec(&spec).expect("render openapi json");
    assert_eq!(
        doc.get("openapi"),
        Some(&serde_json::Value::String("3.2.0".to_string()))
    );

    let server_content =
        &doc["paths"]["/watch"]["get"]["responses"]["200"]["content"]["text/event-stream"];
    assert!(server_content.get("itemSchema").is_some());
    assert!(server_content.get("schema").is_none());

    let client_content =
        &doc["paths"]["/upload"]["post"]["requestBody"]["content"]["application/x-ndjson"];
    assert!(client_content.get("itemSchema").is_some());
    assert!(client_content.get("schema").is_none());
}
