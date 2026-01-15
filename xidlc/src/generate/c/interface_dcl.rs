use crate::error::IdlcResult;
use crate::generate::c::util::c_type;
use crate::generate::c::{CRender, CRenderOutput, CRenderer};
use serde::Serialize;
use std::collections::HashSet;
use xidl_parser::hir;

#[derive(Serialize)]
struct InterfaceContext {
    ident: String,
    operations: Vec<InterfaceOperation>,
}

#[derive(Serialize)]
struct InterfaceOperation {
    name: String,
    return_ty: String,
    params: Vec<InterfaceParam>,
    returns_void: bool,
}

#[derive(Serialize)]
struct InterfaceParam {
    ty: String,
    name: String,
}

impl CRender for hir::InterfaceDcl {
    fn render(&self, renderer: &CRenderer) -> IdlcResult<CRenderOutput> {
        let (ident, body) = match &self.decl {
            hir::InterfaceDclInner::InterfaceForwardDcl(forward) => (forward.ident.clone(), None),
            hir::InterfaceDclInner::InterfaceDef(def) => {
                (def.header.ident.clone(), def.interface_body.as_ref())
            }
        };

        let operations = body
            .map(|body| collect_operations(body))
            .unwrap_or_default();
        let ctx = InterfaceContext { ident, operations };

        let header = renderer.render_template("interface.h.j2", &ctx)?;
        let source = renderer.render_template("interface.c.j2", &ctx)?;
        Ok(CRenderOutput::default()
            .push_header(header)
            .push_source(source))
    }
}

fn collect_operations(body: &hir::InterfaceBody) -> Vec<InterfaceOperation> {
    let mut ops = Vec::new();
    let mut existing = HashSet::new();

    for export in &body.0 {
        if let hir::Export::OpDcl(op) = export {
            existing.insert(op.ident.clone());
        }
    }

    for export in &body.0 {
        match export {
            hir::Export::OpDcl(op) => ops.push(operation_from_op(op)),
            hir::Export::AttrDcl(attr) => ops.extend(operations_from_attr(attr, &existing)),
            _ => {}
        }
    }

    ops
}

fn operation_from_op(op: &hir::OpDcl) -> InterfaceOperation {
    let return_ty = match &op.ty {
        hir::OpTypeSpec::Void => "void".to_string(),
        hir::OpTypeSpec::TypeSpec(ty) => c_type(ty),
    };

    let params = op
        .parameter
        .as_ref()
        .map(|params| params.0.iter().map(param_from_dcl).collect())
        .unwrap_or_else(Vec::new);

    InterfaceOperation {
        name: op.ident.clone(),
        return_ty,
        params,
        returns_void: matches!(op.ty, hir::OpTypeSpec::Void),
    }
}

fn param_from_dcl(param: &hir::ParamDcl) -> InterfaceParam {
    let mode = param
        .attr
        .as_ref()
        .map(|attr| attr.0.as_str())
        .unwrap_or("in");
    let mut ty = c_type(&param.ty);
    if mode == "out" || mode == "inout" {
        ty.push_str(" *");
    }

    InterfaceParam {
        ty,
        name: param.declarator.0.clone(),
    }
}

fn operations_from_attr(
    attr: &hir::AttrDcl,
    existing: &HashSet<String>,
) -> Vec<InterfaceOperation> {
    let attrs = match &attr.decl {
        hir::AttrDclInner::ReadonlyAttrSpec(spec) => readonly_attrs(spec),
        hir::AttrDclInner::AttrSpec(spec) => attrs_with_set(spec),
    };

    let mut ops = Vec::new();
    for attr in attrs {
        let get_name = format!("get_attribute_{}", attr.name);
        if !existing.contains(&get_name) {
            ops.push(InterfaceOperation {
                name: get_name,
                return_ty: c_type(&attr.ty),
                params: Vec::new(),
                returns_void: false,
            });
        }

        if !attr.readonly {
            let set_name = format!("set_attribute_{}", attr.name);
            if !existing.contains(&set_name) {
                ops.push(InterfaceOperation {
                    name: set_name,
                    return_ty: "void".to_string(),
                    params: vec![InterfaceParam {
                        ty: c_type(&attr.ty),
                        name: attr.name.clone(),
                    }],
                    returns_void: true,
                });
            }
        }
    }

    ops
}

struct AttrInfo {
    name: String,
    ty: hir::TypeSpec,
    readonly: bool,
}

fn readonly_attrs(spec: &hir::ReadonlyAttrSpec) -> Vec<AttrInfo> {
    match &spec.declarator {
        hir::ReadonlyAttrDeclarator::SimpleDeclarator(decl) => vec![AttrInfo {
            name: decl.0.clone(),
            ty: spec.ty.clone(),
            readonly: true,
        }],
        hir::ReadonlyAttrDeclarator::RaisesExpr(raises) => {
            let _ = raises;
            Vec::new()
        }
    }
}

fn attrs_with_set(spec: &hir::AttrSpec) -> Vec<AttrInfo> {
    match &spec.declarator {
        hir::AttrDeclarator::SimpleDeclarator(decls) => decls
            .iter()
            .map(|decl| AttrInfo {
                name: decl.0.clone(),
                ty: spec.ty.clone(),
                readonly: false,
            })
            .collect(),
        hir::AttrDeclarator::WithRaises { declarator, raises } => {
            let _ = raises;
            vec![AttrInfo {
                name: declarator.0.clone(),
                ty: spec.ty.clone(),
                readonly: false,
            }]
        }
    }
}
