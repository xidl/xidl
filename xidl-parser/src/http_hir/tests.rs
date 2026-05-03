use std::collections::HashMap;

use crate::hir;

fn parse(source: &str) -> hir::Specification {
    let typed = crate::parser::parser_text(source).expect("parse idl");
    hir::Specification::from_typed_ast_with_properties_and_path(
        typed,
        HashMap::new(),
        std::path::Path::new("input.idl"),
    )
    .expect("project hir")
}

#[test]
fn projects_normalized_http_operations() {
    let spec = parse(
        r#"
            module api {
              interface CityApi {
                @get(path="/cities/{id}{?region}")
                string get_city(@path("id") in string id, @query("region") in string region, @header("X-Trace-Id") in string trace, out string etag);
              };
            };
            "#,
    );
    let doc = super::project(&spec).expect("http hir");
    let op = &doc.interfaces[0].operations[0];
    assert_eq!(op.method, super::HttpMethod::Get);
    assert_eq!(op.routes[0].path, "/cities/{id}");
    assert_eq!(op.routes[0].path_params, vec!["id".to_string()]);
    assert_eq!(op.routes[0].query_params, vec!["region".to_string()]);
    assert_eq!(
        op.request_params
            .iter()
            .find(|param| matches!(param.kind, super::HttpParamKind::Path))
            .map(|param| param.wire_name.as_str()),
        Some("id")
    );
    assert_eq!(
        op.request_params
            .iter()
            .find(|param| matches!(param.kind, super::HttpParamKind::Query))
            .map(|param| param.wire_name.as_str()),
        Some("region")
    );
    assert_eq!(
        op.request_params
            .iter()
            .find(|param| matches!(param.kind, super::HttpParamKind::Header))
            .map(|param| param.wire_name.as_str()),
        Some("X-Trace-Id")
    );
    assert_eq!(op.response_params[0].name, "etag");
}

#[test]
fn projects_attribute_operations_and_document_metadata() {
    let spec = parse(
        r#"
            #pragma xidlc package = "petstore"
            #pragma xidlc service "https://example.com" "prod"
            interface DeviceApi {
              @server_stream readonly attribute string status;
            };
            "#,
    );
    let doc = super::project(&spec).expect("http hir");
    assert_eq!(doc.document.package.as_deref(), Some("petstore"));
    assert_eq!(doc.document.servers[0].base_url, "https://example.com");
    let ops = &doc.interfaces[0].operations;
    assert_eq!(ops.len(), 2);
    assert_eq!(ops[0].source, super::HttpOperationSource::AttributeGet);
    assert_eq!(ops[1].source, super::HttpOperationSource::AttributeWatch);
    assert_eq!(ops[1].stream.kind, Some(super::semantics::HttpStreamKind::Server));
}

#[test]
fn projects_stream_operations_with_http_stream_metadata() {
    let spec = parse(
        r#"
            interface CityFeed {
              @get(path="/cities/feed")
              @server_stream(codec="sse")
              string watch();

              @post(path="/cities/import")
              @client_stream(codec="ndjson")
              void import(in string name);
            };
            "#,
    );
    let doc = super::project(&spec).expect("http hir");
    let ops = &doc.interfaces[0].operations;

    assert_eq!(ops[0].stream.kind, Some(super::semantics::HttpStreamKind::Server));
    assert_eq!(ops[0].stream.codec, super::semantics::HttpStreamCodec::Sse);
    assert_eq!(ops[0].method, super::HttpMethod::Get);

    assert_eq!(ops[1].stream.kind, Some(super::semantics::HttpStreamKind::Client));
    assert_eq!(ops[1].stream.codec, super::semantics::HttpStreamCodec::Ndjson);
    assert_eq!(ops[1].method, super::HttpMethod::Post);
}
