use super::*;
use crate::hir::{
    AttrDcl, AttrDclInner, AttrDeclarator, AttrSpec, ReadonlyAttrDeclarator, ReadonlyAttrSpec,
    SimpleDeclarator, StringType, TypeSpec,
};

fn string_ty() -> TypeSpec {
    TypeSpec::StringType(StringType { bound: None })
}

#[test]
fn project_attr_covers_readonly_readwrite_and_stream_watch_paths() {
    let readonly = AttrDcl {
        annotations: vec![crate::hir::Annotation::Builtin {
            name: "server_stream".to_string(),
            params: None,
        }],
        decl: AttrDclInner::ReadonlyAttrSpec(ReadonlyAttrSpec {
            ty: string_ty(),
            declarator: ReadonlyAttrDeclarator::SimpleDeclarator(SimpleDeclarator(
                "status".to_string(),
            )),
        }),
    };
    let (methods, watches) = project_attr(
        &readonly,
        "DeviceApi",
        &["iot".to_string()],
        &std::collections::HashSet::new(),
    )
    .expect("readonly");
    assert_eq!(methods.len(), 2);
    assert_eq!(watches.len(), 1);
    assert_eq!(methods[0].source, JsonRpcMethodSource::AttributeGet);
    assert_eq!(methods[1].kind, JsonRpcMethodKind::StreamSource);

    let readwrite = AttrDcl {
        annotations: Vec::new(),
        decl: AttrDclInner::AttrSpec(AttrSpec {
            ty: string_ty(),
            declarator: AttrDeclarator::WithRaises {
                declarator: SimpleDeclarator("mode".to_string()),
                raises: crate::hir::AttrRaisesExpr::SetExcepExpr(crate::hir::SetExcepExpr {
                    expr: crate::hir::ExceptionList(Vec::new()),
                }),
            },
        }),
    };
    let (methods, watches) = project_attr(
        &readwrite,
        "DeviceApi",
        &[],
        &std::collections::HashSet::new(),
    )
    .expect("readwrite");
    assert_eq!(methods.len(), 2);
    assert!(watches.is_empty());
    assert_eq!(methods[1].request_fields[0].name, "mode");
    assert_eq!(methods[1].response_kind, JsonRpcResponseKind::Empty);

    let multi = AttrDcl {
        annotations: vec![crate::hir::Annotation::Builtin {
            name: "server_stream".to_string(),
            params: None,
        }],
        decl: AttrDclInner::AttrSpec(AttrSpec {
            ty: string_ty(),
            declarator: AttrDeclarator::SimpleDeclarator(vec![
                SimpleDeclarator("temperature".to_string()),
                SimpleDeclarator("pressure".to_string()),
            ]),
        }),
    };
    let (methods, watches) = project_attr(
        &multi,
        "DeviceApi",
        &["api".to_string()],
        &std::collections::HashSet::new(),
    )
    .expect("multi attr");
    assert_eq!(methods.len(), 6);
    assert_eq!(watches.len(), 2);
    assert_eq!(
        methods[2].source,
        JsonRpcMethodSource::AttributeStreamSource
    );
    assert_eq!(methods[0].response_fields[0].name, "return");

    let readonly_raises = AttrDcl {
        annotations: Vec::new(),
        decl: AttrDclInner::ReadonlyAttrSpec(ReadonlyAttrSpec {
            ty: string_ty(),
            declarator: ReadonlyAttrDeclarator::RaisesExpr(crate::hir::RaisesExpr(Vec::new())),
        }),
    };
    let (methods, watches) = project_attr(
        &readonly_raises,
        "DeviceApi",
        &[],
        &std::collections::HashSet::new(),
    )
    .expect("readonly raises");
    assert!(methods.is_empty());
    assert!(watches.is_empty());
}
