use super::*;
use crate::hir;
use std::collections::HashMap;

fn parse(source: &str) -> hir::Specification {
    let typed = crate::parser::parser_text(source).expect("parse idl");
    let mut properties = HashMap::new();
    properties.insert(
        "expand_interface".to_string(),
        serde_json::Value::Bool(false),
    );
    hir::Specification::from_typed_ast_with_properties(typed, properties)
}

fn string_param(name: &str, kind: HttpParamKind) -> HttpParam {
    HttpParam {
        name: name.to_string(),
        wire_name: name.to_string(),
        ty: hir::TypeSpec::StringType(hir::StringType { bound: None }),
        kind,
        optional: false,
        flatten: false,
    }
}

fn sequence_ty() -> hir::TypeSpec {
    hir::TypeSpec::SequenceType(hir::SequenceType {
        ty: Box::new(hir::TypeSpec::StringType(hir::StringType { bound: None })),
        len: None,
    })
}

#[test]
fn test_build_operation_signature() {
    let spec = parse(
        r#"
        interface TestApi {
            string test_op(@path @rename("id") in string id, @optional in string opt, out string result);
        };
        "#,
    );
    let op = match &spec.0[0] {
        hir::Definition::InterfaceDcl(i) => match &i.decl {
            hir::InterfaceDclInner::InterfaceDef(d) => {
                match &d.interface_body.as_ref().unwrap().0[0] {
                    hir::Export::OpDcl(op) => op,
                    _ => panic!("expected op"),
                }
            }
            _ => panic!("expected interface"),
        },
        _ => panic!("expected interface"),
    };

    let sig = build_operation_signature(op);
    assert_eq!(sig.params.len(), 3);
    assert_eq!(sig.params[0].name, "id");
    assert_eq!(sig.params[0].direction, HttpSignatureParamDirection::In);
    assert!(
        sig.params[0]
            .annotations
            .iter()
            .any(|a| matches!(a, HttpSignatureParamAnnotation::Path { .. }))
    );

    assert_eq!(sig.params[1].name, "opt");
    assert_eq!(sig.params[1].direction, HttpSignatureParamDirection::In);
    assert!(
        sig.params[1]
            .annotations
            .iter()
            .any(|a| matches!(a, HttpSignatureParamAnnotation::Optional))
    );

    assert_eq!(sig.params[2].name, "result");
    assert_eq!(sig.params[2].direction, HttpSignatureParamDirection::Out);

    assert!(matches!(
        sig.return_type.as_ref().unwrap(),
        hir::TypeSpec::StringType(_)
    ));
}

#[test]
fn test_build_http_mapping_single_body() {
    let stream = HttpStreamConfig {
        kind: None,
        codec: HttpStreamCodec::Ndjson,
    };
    let request_params = vec![HttpParam {
        name: "data".to_string(),
        wire_name: "data".to_string(),
        ty: hir::TypeSpec::StringType(hir::StringType { bound: None }),
        kind: HttpParamKind::Body,
        optional: false,
        flatten: false,
    }];
    let response_params = vec![];
    let return_type = Some(hir::TypeSpec::StringType(hir::StringType { bound: None }));

    let mapping = build_http_mapping(
        HttpMethod::Post,
        &stream,
        Some("application/json"),
        Some("application/json"),
        &request_params,
        &response_params,
        &return_type,
    );

    match mapping.request.body.shape {
        HttpRequestBodyShape::SingleValue { source_param, .. } => {
            assert_eq!(source_param, "data");
        }
        _ => panic!("expected SingleValue request body"),
    }

    match mapping.response.body.shape {
        HttpResponseBodyShape::ReturnOnly { .. } => {}
        _ => panic!("expected ReturnOnly response body"),
    }
}

#[test]
fn test_build_http_mapping_object_body() {
    let stream = HttpStreamConfig {
        kind: None,
        codec: HttpStreamCodec::Ndjson,
    };
    let request_params = vec![
        HttpParam {
            name: "p1".to_string(),
            wire_name: "p1".to_string(),
            ty: hir::TypeSpec::StringType(hir::StringType { bound: None }),
            kind: HttpParamKind::Body,
            optional: false,
            flatten: false,
        },
        HttpParam {
            name: "p2".to_string(),
            wire_name: "p2".to_string(),
            ty: hir::TypeSpec::StringType(hir::StringType { bound: None }),
            kind: HttpParamKind::Body,
            optional: true,
            flatten: false,
        },
    ];
    let response_params = vec![HttpParam {
        name: "out1".to_string(),
        wire_name: "out1".to_string(),
        ty: hir::TypeSpec::StringType(hir::StringType { bound: None }),
        kind: HttpParamKind::Body,
        optional: false,
        flatten: false,
    }];
    let return_type = Some(hir::TypeSpec::StringType(hir::StringType { bound: None }));

    let mapping = build_http_mapping(
        HttpMethod::Post,
        &stream,
        Some("application/json"),
        Some("application/json"),
        &request_params,
        &response_params,
        &return_type,
    );

    match mapping.request.body.shape {
        HttpRequestBodyShape::Object { fields } => {
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].source_param, "p1");
            assert_eq!(fields[1].source_param, "p2");
        }
        _ => panic!("expected Object request body"),
    }

    match mapping.response.body.shape {
        HttpResponseBodyShape::Object { fields } => {
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].field_name, "return");
            assert_eq!(fields[1].field_name, "out1");
        }
        _ => panic!("expected Object response body"),
    }
}

#[test]
fn test_build_http_mapping_flatten_body() {
    let stream = HttpStreamConfig {
        kind: None,
        codec: HttpStreamCodec::Ndjson,
    };
    let request_params = vec![HttpParam {
        name: "data".to_string(),
        wire_name: "data".to_string(),
        ty: hir::TypeSpec::StringType(hir::StringType { bound: None }),
        kind: HttpParamKind::Body,
        optional: false,
        flatten: true,
    }];
    let response_params = vec![];
    let return_type = None;

    let mapping = build_http_mapping(
        HttpMethod::Post,
        &stream,
        Some("application/json"),
        Some("application/json"),
        &request_params,
        &response_params,
        &return_type,
    );

    // Flattened single param stays SingleValue with flatten=true
    match mapping.request.body.shape {
        HttpRequestBodyShape::SingleValue {
            source_param,
            flatten,
            ..
        } => {
            assert_eq!(source_param, "data");
            assert!(flatten);
        }
        _ => panic!("expected SingleValue request body for flattened param"),
    }
}

#[test]
fn test_build_http_mapping_defaults_empty_scalar_and_composite_content_types() {
    let stream = HttpStreamConfig {
        kind: None,
        codec: HttpStreamCodec::Ndjson,
    };

    let empty = build_http_mapping(HttpMethod::Post, &stream, None, None, &[], &[], &None);
    assert!(matches!(
        empty.request.body.shape,
        HttpRequestBodyShape::Empty
    ));
    assert_eq!(empty.request.body.content_type, None);
    assert_eq!(empty.request.body.codec, None);
    assert!(matches!(
        empty.response.body.shape,
        HttpResponseBodyShape::Empty
    ));
    assert_eq!(empty.response.body.content_type, None);
    assert_eq!(empty.response.body.codec, None);
    assert_eq!(empty.response.status, "204");

    let scalar = build_http_mapping(
        HttpMethod::Post,
        &stream,
        None,
        None,
        &[string_param("body", HttpParamKind::Body)],
        &[],
        &Some(hir::TypeSpec::StringType(hir::StringType { bound: None })),
    );
    assert_eq!(
        scalar.request.body.content_type.as_deref(),
        Some("text/plain")
    );
    assert_eq!(scalar.request.body.codec, Some(HttpBodyCodec::Text));
    assert_eq!(
        scalar.response.body.content_type.as_deref(),
        Some("text/plain")
    );
    assert_eq!(scalar.response.body.codec, Some(HttpBodyCodec::Text));
    assert_eq!(scalar.response.status, "200");

    let composite = build_http_mapping(
        HttpMethod::Post,
        &stream,
        None,
        None,
        &[HttpParam {
            ty: sequence_ty(),
            ..string_param("items", HttpParamKind::Body)
        }],
        &[],
        &Some(sequence_ty()),
    );
    assert_eq!(
        composite.request.body.content_type.as_deref(),
        Some("application/json")
    );
    assert_eq!(composite.request.body.codec, Some(HttpBodyCodec::Json));
    assert_eq!(
        composite.response.body.content_type.as_deref(),
        Some("application/json")
    );
    assert_eq!(composite.response.body.codec, Some(HttpBodyCodec::Json));
}

#[test]
fn test_build_http_mapping_covers_bindings_streams_and_codecs() {
    let client_stream = HttpStreamConfig {
        kind: Some(HttpStreamKind::Client),
        codec: HttpStreamCodec::Ndjson,
    };
    let mapping = build_http_mapping(
        HttpMethod::Post,
        &client_stream,
        Some("application/msgpack"),
        Some("application/x-custom"),
        &[
            string_param("id", HttpParamKind::Path),
            string_param("q", HttpParamKind::Query),
            string_param("trace", HttpParamKind::Header),
            string_param("session", HttpParamKind::Cookie),
            string_param("item", HttpParamKind::Body),
        ],
        &[
            string_param("etag", HttpParamKind::Header),
            string_param("token", HttpParamKind::Cookie),
            string_param("ignored", HttpParamKind::Query),
        ],
        &None,
    );

    assert_eq!(mapping.request.path[0].source_param, "id");
    assert_eq!(mapping.request.query[0].source_param, "q");
    assert_eq!(mapping.request.header[0].source_param, "trace");
    assert_eq!(mapping.request.cookie[0].source_param, "session");
    assert_eq!(
        mapping.request.body.content_type.as_deref(),
        Some("application/x-ndjson")
    );
    assert_eq!(mapping.request.body.codec, None);
    assert!(matches!(
        mapping.request.body.shape,
        HttpRequestBodyShape::Stream {
            codec: HttpStreamPayloadCodec::Ndjson,
            ..
        }
    ));
    assert_eq!(mapping.response.header[0].wire_name, "etag");
    assert_eq!(mapping.response.cookie[0].wire_name, "token");
    assert_eq!(mapping.response.body.content_type, None);

    let server_stream = HttpStreamConfig {
        kind: Some(HttpStreamKind::Server),
        codec: HttpStreamCodec::Sse,
    };
    let mapping = build_http_mapping(
        HttpMethod::Get,
        &server_stream,
        Some("application/x-www-form-urlencoded"),
        Some("application/msgpack"),
        &[],
        &[string_param("events", HttpParamKind::Body)],
        &None,
    );
    assert_eq!(
        mapping.response.body.content_type.as_deref(),
        Some("text/event-stream")
    );
    assert!(matches!(
        mapping.response.body.shape,
        HttpResponseBodyShape::Stream {
            item_source: HttpOutputSource::Param { .. },
            codec: HttpStreamPayloadCodec::Sse,
            ..
        }
    ));

    let explicit = build_http_mapping(
        HttpMethod::Head,
        &HttpStreamConfig {
            kind: None,
            codec: HttpStreamCodec::Ndjson,
        },
        Some("application/x-www-form-urlencoded"),
        Some("application/msgpack"),
        &[string_param("form", HttpParamKind::Body)],
        &[string_param("value", HttpParamKind::Body)],
        &None,
    );
    assert_eq!(
        explicit.request.body.codec,
        Some(HttpBodyCodec::FormUrlEncoded)
    );
    assert_eq!(explicit.response.body.codec, Some(HttpBodyCodec::Msgpack));
    assert_eq!(explicit.response.status, "204");
}

#[test]
fn test_build_http_mapping_covers_empty_stream_and_single_output_body() {
    let client_stream_without_body = build_http_mapping(
        HttpMethod::Post,
        &HttpStreamConfig {
            kind: Some(HttpStreamKind::Client),
            codec: HttpStreamCodec::Sse,
        },
        None,
        None,
        &[],
        &[],
        &None,
    );
    assert!(matches!(
        client_stream_without_body.request.body.shape,
        HttpRequestBodyShape::Empty
    ));
    assert_eq!(client_stream_without_body.request.body.content_type, None);

    let server_stream_without_body = build_http_mapping(
        HttpMethod::Get,
        &HttpStreamConfig {
            kind: Some(HttpStreamKind::Server),
            codec: HttpStreamCodec::Ndjson,
        },
        None,
        None,
        &[],
        &[],
        &None,
    );
    assert!(matches!(
        server_stream_without_body.response.body.shape,
        HttpResponseBodyShape::Empty
    ));
    assert_eq!(server_stream_without_body.response.status, "204");

    let single_output = build_http_mapping(
        HttpMethod::Post,
        &HttpStreamConfig {
            kind: None,
            codec: HttpStreamCodec::Ndjson,
        },
        None,
        None,
        &[],
        &[string_param("value", HttpParamKind::Body)],
        &None,
    );
    assert!(matches!(
        single_output.response.body.shape,
        HttpResponseBodyShape::SingleValue {
            source: HttpOutputSource::Param { .. },
            ..
        }
    ));
    assert_eq!(
        single_output.response.body.content_type.as_deref(),
        Some("text/plain")
    );
    assert_eq!(single_output.response.status, "200");
}
