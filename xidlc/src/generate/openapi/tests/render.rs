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
    let rest_hir = xidl_parser::rest_hir::project(spec).expect("project http hir");
    render_openapi_json(spec, &rest_hir)
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
        recursive: false,
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

#[test]
fn render_openapi_json_preserves_text_plain_content_types() {
    let spec = parse_spec(
        r#"
        interface PlainTextApi {
          @route(method="POST", path="/echo")
          @Consumes("text/plain")
          @Produces("text/plain")
          string echo(in string body);
        };
        "#,
    );
    let doc = render_openapi_json_from_spec(&spec).expect("render openapi json");

    let request_content = &doc["paths"]["/echo"]["post"]["requestBody"]["content"];
    assert!(request_content.get("text/plain").is_some());
    assert!(request_content.get("application/json").is_none());
    assert_eq!(
        request_content["text/plain"]["schema"]["type"],
        serde_json::Value::String("string".to_string())
    );

    let response_content = &doc["paths"]["/echo"]["post"]["responses"]["200"]["content"];
    assert!(response_content.get("text/plain").is_some());
    assert!(response_content.get("application/json").is_none());
    assert_eq!(
        response_content["text/plain"]["schema"]["type"],
        serde_json::Value::String("string".to_string())
    );
}

#[test]
fn render_openapi_json_uses_produce_alias_for_get_response_content_type() {
    let spec = parse_spec(
        r#"
        interface HttpService {
          @Consume("text/plain")
          @Produce("text/plain")
          @get(path = "/ip")
          string get_ip();
        };
        "#,
    );
    let doc = render_openapi_json_from_spec(&spec).expect("render openapi json");

    let response_content = &doc["paths"]["/ip"]["get"]["responses"]["200"]["content"];
    assert!(response_content.get("text/plain").is_some());
    assert!(response_content.get("application/json").is_none());
    assert_eq!(
        response_content["text/plain"]["schema"]["type"],
        serde_json::Value::String("string".to_string())
    );
}

fn responses_of(
    doc: &serde_json::Value,
    path: &str,
    verb: &str,
) -> std::collections::BTreeMap<String, serde_json::Value> {
    doc["paths"][path][verb]["responses"]
        .as_object()
        .expect("responses object")
        .iter()
        .map(|(status, response)| (status.clone(), response.clone()))
        .collect()
}

#[test]
fn render_openapi_json_emits_500_error_response_for_every_operation() {
    let spec = parse_spec(
        r#"
        interface PlainApi {
          string ping();
        };
        "#,
    );
    let doc = render_openapi_json_from_spec(&spec).expect("render openapi json");

    let responses = responses_of(&doc, "/ping", "post");
    assert!(responses.contains_key("200"));
    assert!(responses.contains_key("500"));
    assert_eq!(
        responses["500"]["content"]["application/json"]["schema"]["$ref"],
        serde_json::Value::String("#/components/schemas/Error".to_string())
    );
}

#[test]
fn render_openapi_json_emits_400_only_for_operations_with_request_contract() {
    let spec = parse_spec(
        r#"
        interface MixedApi {
          string ping();

          string echo(in string message);

          @get(path = "/search")
          string search(@query string q);
        };
        "#,
    );
    let doc = render_openapi_json_from_spec(&spec).expect("render openapi json");

    assert!(!responses_of(&doc, "/ping", "post").contains_key("400"));
    assert!(responses_of(&doc, "/echo", "post").contains_key("400"));
    assert!(responses_of(&doc, "/search", "get").contains_key("400"));
}

#[test]
fn render_openapi_json_emits_401_only_for_operations_with_security() {
    let spec = parse_spec(
        r#"
        interface SecuredApi {
          @http_basic
          string secret();

          @no_security
          string open();

          string plain();
        };
        "#,
    );
    let doc = render_openapi_json_from_spec(&spec).expect("render openapi json");

    assert!(responses_of(&doc, "/secret", "post").contains_key("401"));
    assert!(!responses_of(&doc, "/open", "post").contains_key("401"));
    assert!(!responses_of(&doc, "/plain", "post").contains_key("401"));
}

#[test]
fn render_openapi_json_tags_operations_by_interface_name() {
    let spec = parse_spec(
        r#"
        interface AlphaApi {
          string ping();
        };

        interface BetaApi {
          @get(path = "/beta")
          string fetch();
        };
        "#,
    );
    let doc = render_openapi_json_from_spec(&spec).expect("render openapi json");

    assert_eq!(
        doc["tags"],
        serde_json::json!([
            { "name": "AlphaApi" },
            { "name": "BetaApi" }
        ])
    );
    assert_eq!(
        doc["paths"]["/ping"]["post"]["tags"],
        serde_json::json!(["AlphaApi"])
    );
    assert_eq!(
        doc["paths"]["/beta"]["get"]["tags"],
        serde_json::json!(["BetaApi"])
    );
}
