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
fn projects_unary_method_request_and_result_shapes() {
    let spec = parse(
        r#"
        module math {
          interface Calc {
            long add(in long a, in long b, out long sum);
          };
        };
        "#,
    );
    let doc = super::project(&spec).expect("jsonrpc hir");
    let method = &doc.interfaces[0].methods[0];
    assert_eq!(method.rpc_name, "math.Calc.add");
    assert_eq!(method.request_fields.len(), 2);
    assert_eq!(method.response_fields.len(), 2);
    assert_eq!(
        method.response_kind,
        super::JsonRpcResponseKind::MultiOutput
    );
}

#[test]
fn projects_attribute_implied_methods_and_watch_helpers() {
    let spec = parse(
        r#"
        interface Device {
          @server_stream readonly attribute string status;
        };
        "#,
    );
    let doc = super::project(&spec).expect("jsonrpc hir");
    let interface = &doc.interfaces[0];
    assert_eq!(interface.methods.len(), 2);
    assert_eq!(
        interface.methods[0].source,
        super::JsonRpcMethodSource::AttributeGet
    );
    assert_eq!(
        interface.methods[1].kind,
        super::JsonRpcMethodKind::StreamSource
    );
    assert_eq!(
        interface.watch_methods[0].stream_rpc_name,
        "Device.set_attribute_status"
    );
}

#[test]
fn projects_stream_method_kinds() {
    let spec = parse(
        r#"
        interface Feed {
          @server_stream string watch();
          @client_stream void upload(in string value);
          @bidi_stream void chat(in string input, out string output);
        };
        "#,
    );
    let doc = super::project(&spec).expect("jsonrpc hir");
    let methods = &doc.interfaces[0].methods;
    assert_eq!(methods[0].kind, super::JsonRpcMethodKind::ServerStream);
    assert_eq!(methods[1].kind, super::JsonRpcMethodKind::ClientStream);
    assert_eq!(methods[2].kind, super::JsonRpcMethodKind::BidiStream);
}
