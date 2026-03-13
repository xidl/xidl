use crate::error::IdlcResult;
use crate::generate::rust::util::rust_type;
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use crate::generate::utils::doc_lines_from_annotations;
use serde_json::json;
use xidl_parser::hir;

impl RustRender for hir::InterfaceDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        match &self.decl {
            hir::InterfaceDclInner::InterfaceForwardDcl(forward) => forward.render(renderer),
            hir::InterfaceDclInner::InterfaceDef(def) => render_interface_def_with_doc(
                def,
                doc_lines_from_annotations(&self.annotations),
                renderer,
            ),
        }
    }
}

impl RustRender for hir::InterfaceForwardDcl {
    fn render(&self, _renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        Ok(RustRenderOutput::empty())
    }
}

impl RustRender for hir::InterfaceDef {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        render_interface_def_with_doc(self, Vec::new(), renderer)
    }
}

fn render_interface_def_with_doc(
    def: &hir::InterfaceDef,
    doc: Vec<String>,
    renderer: &RustRenderer,
) -> IdlcResult<RustRenderOutput> {
    let mut out = RustRenderOutput::default();
    let method_renderer = InterfaceMethodRenderer::new();
    let mut methods = Vec::new();

    if let Some(body) = &def.interface_body {
        for export in &body.0 {
            match export {
                hir::Export::OpDcl(op) => {
                    methods.push(method_renderer.render_op(op));
                }
                hir::Export::AttrDcl(attr) => {
                    methods.extend(method_renderer.render_attr(attr));
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
        "ident": crate::generate::rust::util::rust_ident(&def.header.ident),
        "methods": methods,
        "doc": doc,
    });
    out.extend(renderer.render_source_template("interface.rs.j2", &ctx)?);
    Ok(out)
}

struct InterfaceMethodRenderer {
    type_policy: RustTypePolicy,
}

impl InterfaceMethodRenderer {
    fn new() -> Self {
        Self {
            type_policy: RustTypePolicy,
        }
    }

    fn render_op(&self, op: &hir::OpDcl) -> serde_json::Value {
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
            let ty = self.type_policy.param_type(&param.ty, param.attr.as_ref());
            let name = crate::generate::rust::util::rust_ident(&param.declarator.0);
            param_list.push(format!("{name}: {ty}"));
        }
        self.method_json(
            crate::generate::rust::util::rust_ident(&op.ident),
            ret,
            param_list,
            &doc_lines_from_annotations(&op.annotations),
        )
    }

    fn render_attr(&self, attr: &hir::AttrDcl) -> Vec<serde_json::Value> {
        let doc = doc_lines_from_annotations(&attr.annotations);
        match &attr.decl {
            hir::AttrDclInner::ReadonlyAttrSpec(spec) => self
                .readonly_attr_names(spec)
                .into_iter()
                .map(|name| {
                    self.method_json(
                        name,
                        self.type_policy.return_type(&spec.ty),
                        Vec::new(),
                        &doc,
                    )
                })
                .collect(),
            hir::AttrDclInner::AttrSpec(spec) => {
                let mut out = Vec::new();
                match &spec.declarator {
                    hir::AttrDeclarator::SimpleDeclarator(list) => {
                        for decl in list {
                            let name = crate::generate::rust::util::rust_ident(&decl.0);
                            out.extend(self.render_accessor_methods(&name, &spec.ty, &doc));
                        }
                    }
                    hir::AttrDeclarator::WithRaises { declarator, .. } => {
                        let name = crate::generate::rust::util::rust_ident(&declarator.0);
                        out.extend(self.render_accessor_methods(&name, &spec.ty, &doc));
                    }
                }
                out
            }
        }
    }

    fn readonly_attr_names(&self, spec: &hir::ReadonlyAttrSpec) -> Vec<String> {
        match &spec.declarator {
            hir::ReadonlyAttrDeclarator::SimpleDeclarator(decl) => {
                vec![crate::generate::rust::util::rust_ident(&decl.0)]
            }
            hir::ReadonlyAttrDeclarator::RaisesExpr(_raises) => Vec::new(),
        }
    }

    fn render_accessor_methods(
        &self,
        name: &str,
        ty: &hir::TypeSpec,
        doc: &[String],
    ) -> Vec<serde_json::Value> {
        let ret = self.type_policy.return_type(ty);
        let param = self.type_policy.param_type(ty, None);
        vec![
            self.method_json(name.to_string(), ret, Vec::new(), doc),
            self.method_json(
                format!("set_{name}"),
                "()".to_string(),
                vec![format!("value: {param}")],
                doc,
            ),
        ]
    }

    fn method_json(
        &self,
        name: String,
        ret: String,
        params: Vec<String>,
        doc: &[String],
    ) -> serde_json::Value {
        json!({
            "ret": ret,
            "name": name,
            "params": params,
            "doc": doc,
        })
    }
}

struct RustTypePolicy;

impl RustTypePolicy {
    fn return_type(&self, ty: &hir::TypeSpec) -> String {
        if self.is_value_type(ty) {
            rust_type(ty)
        } else {
            format!("&{}", rust_type(ty))
        }
    }

    fn param_type(&self, ty: &hir::TypeSpec, attr: Option<&hir::ParamAttribute>) -> String {
        match attr.map(|value| value.0.as_str()) {
            Some("out") | Some("inout") => format!("&mut {}", rust_type(ty)),
            _ => self.return_type(ty),
        }
    }

    fn is_value_type(&self, ty: &hir::TypeSpec) -> bool {
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
}
