use crate::error::IdlcResult;
use crate::generate::rust::util::rust_type;
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use serde_json::json;
use xidl_parser::hir;

impl RustRender for hir::InterfaceDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        match &self.decl {
            hir::InterfaceDclInner::InterfaceForwardDcl(forward) => forward.render(renderer),
            hir::InterfaceDclInner::InterfaceDef(def) => def.render(renderer),
        }
    }
}

impl RustRender for hir::InterfaceForwardDcl {
    fn render(&self, _renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        Ok(RustRenderOutput::default())
    }
}

impl RustRender for hir::InterfaceDef {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        let mut out = RustRenderOutput::default();
        let mut methods = Vec::new();

        if let Some(body) = &self.interface_body {
            for export in &body.0 {
                match export {
                    hir::Export::OpDcl(op) => {
                        methods.push(render_op(op));
                    }
                    hir::Export::AttrDcl(attr) => {
                        methods.extend(render_attr(attr));
                    }
                    hir::Export::TypeDcl(type_dcl) => {
                        out.extend(type_dcl.render(renderer)?);
                    }
                    hir::Export::ConstDcl(const_dcl) => {
                        out.extend(const_dcl.render(renderer)?);
                    }
                    hir::Export::ExceptDcl(except) => {
                        out.extend(except.render(renderer)?);
                    }
                }
            }
        }

        let ctx = json!({
            "ident": crate::generate::rust::util::rust_ident(&self.header.ident),
            "methods": methods,
        });
        let rendered = renderer.render_template("interface.rs.j2", &ctx)?;
        out.source.push(rendered);
        Ok(out)
    }
}

fn render_op(op: &hir::OpDcl) -> serde_json::Value {
    let ret = match &op.ty {
        hir::OpTypeSpec::Void => "()".to_string(),
        hir::OpTypeSpec::TypeSpec(ty) => rust_type(ty),
    };
    let params = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);
    let mut param_list = Vec::new();
    for param in params {
        let ty = render_param_type(&param.ty, param.attr.as_ref());
        let name = crate::generate::rust::util::rust_ident(&param.declarator.0);
        param_list.push(format!("{name}: {ty}"));
    }
    json!({
        "ret": ret,
        "name": crate::generate::rust::util::rust_ident(&op.ident),
        "params": param_list,
    })
}

fn render_attr(attr: &hir::AttrDcl) -> Vec<serde_json::Value> {
    match &attr.decl {
        hir::AttrDclInner::ReadonlyAttrSpec(spec) => readonly_attr_names(spec)
            .into_iter()
            .map(|name| {
                let ret = attr_return_type(&spec.ty);
                json!({
                    "ret": ret,
                    "name": name,
                    "params": Vec::<String>::new(),
                })
            })
            .collect(),
        hir::AttrDclInner::AttrSpec(spec) => {
            let mut out = Vec::new();
            match &spec.declarator {
                hir::AttrDeclarator::SimpleDeclarator(list) => {
                    for decl in list {
                        let name = crate::generate::rust::util::rust_ident(&decl.0);
                        let ret = attr_return_type(&spec.ty);
                        let param = render_param_type(&spec.ty, None);
                        out.push(json!({
                            "ret": ret,
                            "name": name.clone(),
                            "params": Vec::<String>::new(),
                        }));
                        out.push(json!({
                            "ret": "()",
                            "name": format!("set_{name}"),
                            "params": vec![format!("value: {param}")],
                        }));
                    }
                }
                hir::AttrDeclarator::WithRaises { declarator, .. } => {
                    let name = crate::generate::rust::util::rust_ident(&declarator.0);
                    let ret = attr_return_type(&spec.ty);
                    let param = render_param_type(&spec.ty, None);
                    out.push(json!({
                        "ret": ret,
                        "name": name.clone(),
                        "params": Vec::<String>::new(),
                    }));
                    out.push(json!({
                        "ret": "()",
                        "name": format!("set_{name}"),
                        "params": vec![format!("value: {param}")],
                    }));
                }
            }
            out
        }
    }
}

fn readonly_attr_names(spec: &hir::ReadonlyAttrSpec) -> Vec<String> {
    match &spec.declarator {
        hir::ReadonlyAttrDeclarator::SimpleDeclarator(decl) => {
            vec![crate::generate::rust::util::rust_ident(&decl.0)]
        }
        hir::ReadonlyAttrDeclarator::RaisesExpr(_raises) => Vec::new(),
    }
}

fn attr_return_type(ty: &hir::TypeSpec) -> String {
    if is_value_type(ty) {
        rust_type(ty)
    } else {
        format!("&{}", rust_type(ty))
    }
}

fn render_param_type(ty: &hir::TypeSpec, attr: Option<&hir::ParamAttribute>) -> String {
    match attr.map(|value| value.0.as_str()) {
        Some("out") | Some("inout") => format!("&mut {}", rust_type(ty)),
        _ => {
            if is_value_type(ty) {
                rust_type(ty)
            } else {
                format!("&{}", rust_type(ty))
            }
        }
    }
}

fn is_value_type(ty: &hir::TypeSpec) -> bool {
    matches!(
        ty,
        hir::TypeSpec::SimpleTypeSpec(
            hir::SimpleTypeSpec::IntegerType(_)
                | hir::SimpleTypeSpec::FloatingPtType
                | hir::SimpleTypeSpec::CharType
                | hir::SimpleTypeSpec::WideCharType
                | hir::SimpleTypeSpec::Boolean
        )
    )
}
