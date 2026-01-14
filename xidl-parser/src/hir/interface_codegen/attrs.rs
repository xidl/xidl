use super::ops::{OperationInfo, ParamInfo, ParamMode, ReturnType};
use super::*;

pub fn operations_from_attr(attr: &AttrDcl, existing: &std::collections::HashSet<String>) -> Vec<OperationInfo> {
    let attrs = match attr {
        AttrDcl::ReadonlyAttrSpec(spec) => readonly_attrs(spec),
        AttrDcl::AttrSpec(spec) => attrs_with_set(spec),
    };

    let mut ops = Vec::new();
    for attr in attrs {
        let get_name = format!("get_attribute_{}", attr.name);
        if !existing.contains(&get_name) {
            ops.push(OperationInfo {
                name: get_name,
                return_ty: ReturnType::Type(attr.ty.clone()),
                params: Vec::new(),
                raises: attr.get_raises,
            });
        }

        if !attr.readonly {
            let set_name = format!("set_attribute_{}", attr.name);
            if !existing.contains(&set_name) {
                ops.push(OperationInfo {
                    name: set_name,
                    return_ty: ReturnType::Void,
                    params: vec![ParamInfo {
                        name: attr.name.clone(),
                        mode: ParamMode::In,
                        ty: attr.ty.clone(),
                    }],
                    raises: attr.set_raises,
                });
            }
        }
    }

    ops
}

struct AttrInfo {
    name: String,
    ty: TypeSpec,
    get_raises: Vec<ScopedName>,
    set_raises: Vec<ScopedName>,
    readonly: bool,
}

fn readonly_attrs(spec: &ReadonlyAttrSpec) -> Vec<AttrInfo> {
    match &spec.declarator {
        ReadonlyAttrDeclarator::SimpleDeclarator(decl) => vec![AttrInfo {
            name: decl.0.clone(),
            ty: spec.ty.clone(),
            get_raises: Vec::new(),
            set_raises: Vec::new(),
            readonly: true,
        }],
        ReadonlyAttrDeclarator::RaisesExpr(raises) => {
            let _ = raises;
            Vec::new()
        }
    }
}

fn attrs_with_set(spec: &AttrSpec) -> Vec<AttrInfo> {
    match &spec.declarator {
        AttrDeclarator::SimpleDeclarator(decls) => decls
            .iter()
            .map(|decl| AttrInfo {
                name: decl.0.clone(),
                ty: spec.ty.clone(),
                get_raises: Vec::new(),
                set_raises: Vec::new(),
                readonly: false,
            })
            .collect(),
        AttrDeclarator::WithRaises { declarator, raises } => {
            let (get_raises, set_raises) = match raises {
                AttrRaisesExpr::Case1(get, set) => (
                    get.expr.0.clone(),
                    set.as_ref().map(|value| value.expr.0.clone()).unwrap_or_default(),
                ),
                AttrRaisesExpr::SetExcepExpr(set) => (Vec::new(), set.expr.0.clone()),
            };

            vec![AttrInfo {
                name: declarator.0.clone(),
                ty: spec.ty.clone(),
                get_raises,
                set_raises,
                readonly: false,
            }]
        }
    }
}
