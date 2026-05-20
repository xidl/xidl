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
                string get_city(@path @rename("id") in string id, @query @rename("region") in string region, @header @rename("X-Trace-Id") in string trace, out string etag);
              };
            };
            "#,
    );
    let doc = super::project(&spec).expect("http hir");
    let op = &doc.interfaces[0].operations[0];
    assert_eq!(op.meta.method, super::HttpMethod::Get);
    assert_eq!(op.meta.routes[0].path, "/cities/{id}");
    assert_eq!(op.meta.routes[0].path_params, vec!["id".to_string()]);
    assert_eq!(op.meta.routes[0].query_params, vec!["region".to_string()]);
    assert_eq!(
        op.http
            .request
            .path
            .iter()
            .find(|b| b.source_param == "id")
            .map(|b| b.wire_name.as_str()),
        Some("id")
    );
    assert_eq!(
        op.http
            .request
            .query
            .iter()
            .find(|b| b.source_param == "region")
            .map(|b| b.wire_name.as_str()),
        Some("region")
    );
    assert_eq!(
        op.http
            .request
            .header
            .iter()
            .find(|b| b.source_param == "trace")
            .map(|b| b.wire_name.as_str()),
        Some("X-Trace-Id")
    );
    assert_eq!(op.signature.params[3].name, "etag");
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
    assert_eq!(ops[0].meta.source, super::HttpOperationSource::AttributeGet);
    assert_eq!(
        ops[1].meta.source,
        super::HttpOperationSource::AttributeWatch
    );
    assert_eq!(
        ops[1].meta.stream.kind,
        Some(super::semantics::HttpStreamKind::Server)
    );
    let empty = parse(
        r#"
            #pragma xidlc package ""
            interface EmptyApi {
              string ping();
            };
            "#,
    );
    let empty_doc = super::project(&empty).expect("empty package");
    assert_eq!(empty_doc.document.package, None);
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

    assert_eq!(
        ops[0].meta.stream.kind,
        Some(super::semantics::HttpStreamKind::Server)
    );
    assert_eq!(
        ops[0].meta.stream.codec,
        super::semantics::HttpStreamCodec::Sse
    );
    assert_eq!(ops[0].meta.method, super::HttpMethod::Get);

    assert_eq!(
        ops[1].meta.stream.kind,
        Some(super::semantics::HttpStreamKind::Client)
    );
    assert_eq!(
        ops[1].meta.stream.codec,
        super::semantics::HttpStreamCodec::Ndjson
    );
    assert_eq!(ops[1].meta.method, super::HttpMethod::Post);
}

#[test]
fn projects_default_routes_skips_forward_decls_and_reads_version_pragma() {
    let spec = parse(
        r#"
            #pragma xidlc openapi version = "2026.04"
            interface FutureApi;
            module api {
              interface PingApi {
                string ping();
              };
            };
            "#,
    );
    let doc = super::project(&spec).expect("http hir");
    assert_eq!(doc.document.version.as_deref(), Some("2026.04"));
    assert_eq!(doc.interfaces.len(), 1);
    assert_eq!(doc.interfaces[0].name, "PingApi");
    assert_eq!(doc.interfaces[0].module_path, vec!["api".to_string()]);
    assert_eq!(
        doc.interfaces[0].operations[0].meta.method,
        super::HttpMethod::Post
    );
    assert_eq!(doc.interfaces[0].operations[0].meta.routes[0].path, "/ping");
}

#[test]
fn rejects_cors_on_attributes() {
    let spec = parse(
        r#"
            interface DeviceApi {
              @cors("https://app.example.com")
              readonly attribute string status;
            };
            "#,
    );
    let err = super::project(&spec).expect_err("attribute cors should fail");
    assert!(
        err.to_string()
            .contains("@cors is only supported on interfaces and methods")
    );
}

#[test]
fn surfaces_route_parse_errors_from_projection() {
    let spec = parse(
        r#"
            interface BrokenApi {
              @get(path="/broken/{id")
              string broken(in string id);
            };
            "#,
    );
    let err = super::project(&spec).expect_err("invalid route should fail");
    assert!(err.to_string().contains("unmatched '{'"));
}
