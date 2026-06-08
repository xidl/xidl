use super::*;
use crate::hir::{
    AttrDcl, AttrDclInner, AttrDeclarator, AttrSpec, ReadonlyAttrDeclarator, ReadonlyAttrSpec,
    SimpleDeclarator, StringType, TypeSpec,
};

fn string_ty() -> TypeSpec {
    TypeSpec::StringType(StringType { bound: None })
}

#[test]
fn projects_readonly_and_watch_attributes() {
    let attr = AttrDcl {
        annotations: Vec::new(),
        decl: AttrDclInner::ReadonlyAttrSpec(ReadonlyAttrSpec {
            ty: string_ty(),
            declarator: ReadonlyAttrDeclarator::SimpleDeclarator(SimpleDeclarator(
                "statusFlag".to_string(),
            )),
        }),
    };
    let ops = project_attribute("DeviceApi", &["iot".to_string()], &attr, None, None, true);
    assert_eq!(ops.len(), 2);
    assert_eq!(ops[0].meta.source, HttpOperationSource::AttributeGet);
    assert_eq!(ops[0].meta.routes[0].path, "/attribute/statusFlag");
    assert_eq!(ops[1].meta.source, HttpOperationSource::AttributeWatch);
    assert_eq!(ops[1].meta.stream.kind, Some(HttpStreamKind::Server));
}

#[test]
fn projects_readwrite_and_with_raises_attributes() {
    let attr = AttrDcl {
        annotations: Vec::new(),
        decl: AttrDclInner::AttrSpec(AttrSpec {
            ty: string_ty(),
            declarator: AttrDeclarator::SimpleDeclarator(vec![
                SimpleDeclarator("temperatureCelsius".to_string()),
                SimpleDeclarator("pressure".to_string()),
            ]),
        }),
    };
    let ops = project_attribute("SensorApi", &["api".to_string()], &attr, None, None, false);
    assert_eq!(ops.len(), 4);
    assert_eq!(ops[1].meta.source, HttpOperationSource::AttributeSet);
    assert_eq!(ops[1].signature.params[0].name, "temperature_celsius");
    assert_eq!(
        ops[1].http.request.body.content_type.as_deref(),
        Some("text/plain")
    );

    let with_raises = AttrDcl {
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
    let ops = project_attribute("SensorApi", &[], &with_raises, None, None, true);
    assert_eq!(ops.len(), 3);
    assert_eq!(ops[2].meta.name, "watch_attribute_mode");
    assert_eq!(ops[2].meta.operation_id, "SensorApi.watch_attribute_mode");
}
