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
