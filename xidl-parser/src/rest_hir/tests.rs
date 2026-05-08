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
    assert_eq!(
        ops[1].stream.kind,
        Some(super::semantics::HttpStreamKind::Server)
    );
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
        ops[0].stream.kind,
        Some(super::semantics::HttpStreamKind::Server)
    );
    assert_eq!(ops[0].stream.codec, super::semantics::HttpStreamCodec::Sse);
    assert_eq!(ops[0].method, super::HttpMethod::Get);
    assert_eq!(ops[0].response_content_type, "text/event-stream");
    assert_eq!(ops[0].response_shape, super::HttpResponseShape::ReturnOnly);

    assert_eq!(
        ops[1].stream.kind,
        Some(super::semantics::HttpStreamKind::Client)
    );
    assert_eq!(
        ops[1].stream.codec,
        super::semantics::HttpStreamCodec::Ndjson
    );
    assert_eq!(ops[1].method, super::HttpMethod::Post);
    assert_eq!(ops[1].request_content_type, "application/x-ndjson");
    assert_eq!(ops[1].request_shape, super::HttpRequestShape::Object);
}

#[test]
fn projects_content_types_based_on_type_complexity() {
    let spec = parse(
        r#"
            struct MyStruct { string a; };
            interface TypeApi {
              @post void op1(in string a);
              @post void op2(in MyStruct a);
              @get string op3();
              @get MyStruct op4();
            };
            "#,
    );
    let doc = super::project(&spec).expect("http hir");
    let ops = &doc.interfaces[0].operations;

    // op1: string input -> text/plain
    assert_eq!(ops[0].request_content_type, "text/plain");
    // op2: struct input -> application/json
    assert_eq!(ops[1].request_content_type, "application/json");
    // op3: string return -> text/plain
    assert_eq!(ops[2].response_content_type, "text/plain");
    // op4: struct return -> application/json
    assert_eq!(ops[3].response_content_type, "application/json");

    let spec2 = parse(
        r#"
            interface TypeApi2 {
              @post string op5(in string a, out string b);
            };
            "#,
    );
    let doc2 = super::project(&spec2).expect("http hir");
    let ops2 = &doc2.interfaces[0].operations;
    // op5: multiple body results -> application/json
    assert_eq!(ops2[0].response_content_type, "application/json");

    let spec3 = parse(
        r#"
            interface TypeApi3 {
              @post void op6(@flatten in string a);
              @post void op7(in string a, in string b);
            };
            "#,
    );
    let doc3 = super::project(&spec3).expect("http hir");
    let ops3 = &doc3.interfaces[0].operations;
    // op6: flatten -> SingleFlattened
    assert_eq!(
        ops3[0].request_body_shape,
        super::HttpBodyShape::SingleFlattened
    );
    // op7: multiple -> Object
    assert_eq!(ops3[1].request_body_shape, super::HttpBodyShape::Object);

    let spec4 = parse(
        r#"
            #pragma xidlc openapi version "3.1.0"
            interface Empty {};
            local interface LocalOnly {};
        "#,
    );
    let doc4 = super::project(&spec4).expect("http hir");
    assert_eq!(doc4.interfaces.len(), 2);
    assert_eq!(doc4.document.version.as_deref(), Some("3.1.0"));
}
