use super::*;
use crate::hir::{
    Annotation, InterfaceBody, InterfaceDcl, InterfaceDclInner, InterfaceDef, InterfaceHeader,
    OpDcl, OpTypeSpec, ParamAttribute, ParamDcl, ParameterDcls, SimpleDeclarator, Specification,
    StringType, TypeSpec,
};

fn string_ty() -> TypeSpec {
    TypeSpec::StringType(StringType { bound: None })
}

fn param(name: &str, attr: Option<&str>) -> ParamDcl {
    ParamDcl {
        annotations: Vec::new(),
        attr: attr.map(|value| ParamAttribute(value.to_string())),
        ty: string_ty(),
        declarator: SimpleDeclarator(name.to_string()),
    }
}

#[test]
fn project_helpers_cover_request_response_and_interface_collection() {
    let op = OpDcl {
        annotations: vec![Annotation::Builtin {
            name: "server_stream".to_string(),
            params: None,
        }],
        ty: OpTypeSpec::TypeSpec(string_ty()),
        ident: "watch".to_string(),
        parameter: Some(ParameterDcls(vec![
            param("id", None),
            param("etag", Some("out")),
            param("both", Some("inout")),
        ])),
        raises: None,
    };
    assert_eq!(
        request_fields(op.parameter.as_ref().unwrap().0.as_slice()).len(),
        2
    );
    assert_eq!(
        response_fields(&op, op.parameter.as_ref().unwrap().0.as_slice()).len(),
        3
    );
    assert_eq!(
        response_kind(&op, op.parameter.as_ref().unwrap().0.as_slice()),
        JsonRpcResponseKind::MultiOutput
    );

    let interface = InterfaceDcl {
        annotations: Vec::new(),
        decl: InterfaceDclInner::InterfaceDef(InterfaceDef {
            header: InterfaceHeader {
                ident: "CityApi".to_string(),
                parent: None,
            },
            interface_body: Some(InterfaceBody(vec![crate::hir::Export::OpDcl(op)])),
        }),
    };
    let spec = Specification(vec![crate::hir::Definition::InterfaceDcl(interface)]);
    let doc = project(&spec).expect("project");
    assert_eq!(doc.interfaces.len(), 1);
    assert_eq!(
        doc.interfaces[0].methods[0].kind,
        JsonRpcMethodKind::ServerStream
    );

    let forward = InterfaceDcl {
        annotations: Vec::new(),
        decl: InterfaceDclInner::InterfaceForwardDcl(crate::hir::InterfaceForwardDcl {
            ident: "ForwardOnly".to_string(),
        }),
    };
    let projected = project_interface(&forward, &["api".to_string()]).expect("forward");
    assert_eq!(projected.ident, "ForwardOnly");
    assert!(projected.methods.is_empty());
}
