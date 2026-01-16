use crate::error::IdlcResult;
use crate::generate::cpp::util::{cpp_scoped_name, cpp_type};
use crate::generate::cpp::{CppRender, CppRenderOutput, CppRenderer};
use serde_json::json;
use xidl_parser::hir;

impl CppRender for hir::InterfaceDcl {
    fn render(&self, renderer: &CppRenderer) -> IdlcResult<CppRenderOutput> {
        match &self.decl {
            hir::InterfaceDclInner::InterfaceForwardDcl(forward) => forward.render(renderer),
            hir::InterfaceDclInner::InterfaceDef(def) => def.render(renderer),
        }
    }
}

impl CppRender for hir::InterfaceForwardDcl {
    fn render(&self, renderer: &CppRenderer) -> IdlcResult<CppRenderOutput> {
        let ctx = json!({ "kind": "class", "ident": &self.ident });
        let rendered = renderer.render_template("forward.h.j2", &ctx)?;
        Ok(CppRenderOutput::default().push_header(rendered))
    }
}

impl CppRender for hir::InterfaceDef {
    fn render(&self, renderer: &CppRenderer) -> IdlcResult<CppRenderOutput> {
        let mut out = CppRenderOutput::default();

        let parents = self
            .header
            .parent
            .as_ref()
            .map(|value| {
                value
                    .0
                    .iter()
                    .map(|parent| cpp_scoped_name(&parent.0))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let mut methods = Vec::new();
        let mut nested_lines = Vec::new();
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
                        let rendered = type_dcl.render(renderer)?;
                        for header in rendered.header {
                            nested_lines.extend(header.lines().map(|line| line.to_string()));
                        }
                        out.source.extend(rendered.source);
                    }
                    hir::Export::ConstDcl(const_dcl) => {
                        let rendered = const_dcl.render(renderer)?;
                        for header in rendered.header {
                            nested_lines.extend(header.lines().map(|line| line.to_string()));
                        }
                        out.source.extend(rendered.source);
                    }
                    hir::Export::ExceptDcl(except) => {
                        let rendered = except.render(renderer)?;
                        for header in rendered.header {
                            nested_lines.extend(header.lines().map(|line| line.to_string()));
                        }
                    }
                }
            }
        }

        let ctx = json!({
            "ident": &self.header.ident,
            "parents": parents,
            "methods": methods,
            "nested_lines": nested_lines,
        });
        let rendered = renderer.render_template("interface.h.j2", &ctx)?;
        out.header.push(rendered);
        Ok(out)
    }
}

fn render_op(op: &hir::OpDcl) -> serde_json::Value {
    let ret = match &op.ty {
        hir::OpTypeSpec::Void => "void".to_string(),
        hir::OpTypeSpec::TypeSpec(ty) => cpp_type(ty),
    };
    let params = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);
    let mut param_list = Vec::new();
    for param in params {
        let ty = render_param_type(&param.ty, param.attr.as_ref());
        let name = &param.declarator.0;
        param_list.push(format!("{ty} {name}"));
    }
    json!({
        "ret": ret,
        "name": &op.ident,
        "params": param_list,
        "is_const": false,
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
                    "is_const": true,
                })
            })
            .collect(),
        hir::AttrDclInner::AttrSpec(spec) => {
            let mut out = Vec::new();
            match &spec.declarator {
                hir::AttrDeclarator::SimpleDeclarator(list) => {
                    for decl in list {
                        let name = decl.0.as_str();
                        let ret = attr_return_type(&spec.ty);
                        let param = render_param_type(&spec.ty, None);
                        out.push(json!({
                            "ret": ret,
                            "name": name,
                            "params": Vec::<String>::new(),
                            "is_const": true,
                        }));
                        out.push(json!({
                            "ret": "void",
                            "name": name,
                            "params": vec![format!("{param} value")],
                            "is_const": false,
                        }));
                    }
                }
                hir::AttrDeclarator::WithRaises { declarator, .. } => {
                    let name = declarator.0.as_str();
                    let ret = attr_return_type(&spec.ty);
                    let param = render_param_type(&spec.ty, None);
                    out.push(json!({
                        "ret": ret,
                        "name": name,
                        "params": Vec::<String>::new(),
                        "is_const": true,
                    }));
                    out.push(json!({
                        "ret": "void",
                        "name": name,
                        "params": vec![format!("{param} value")],
                        "is_const": false,
                    }));
                }
            }
            out
        }
    }
}

fn readonly_attr_names(spec: &hir::ReadonlyAttrSpec) -> Vec<String> {
    match &spec.declarator {
        hir::ReadonlyAttrDeclarator::SimpleDeclarator(decl) => vec![decl.0.clone()],
        hir::ReadonlyAttrDeclarator::RaisesExpr(_raises) => Vec::new(),
    }
}

fn attr_return_type(ty: &hir::TypeSpec) -> String {
    if is_value_type(ty) {
        cpp_type(ty)
    } else {
        format!("const {}&", cpp_type(ty))
    }
}

fn render_param_type(ty: &hir::TypeSpec, attr: Option<&hir::ParamAttribute>) -> String {
    match attr.map(|value| value.0.as_str()) {
        Some("out") | Some("inout") => format!("{}&", cpp_type(ty)),
        _ => {
            if is_value_type(ty) {
                cpp_type(ty)
            } else {
                format!("const {}&", cpp_type(ty))
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
