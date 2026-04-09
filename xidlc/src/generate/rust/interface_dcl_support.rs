use crate::generate::rust::util::rust_passthrough_attrs_from_annotations;
use crate::generate::rust::util::rust_type;
use crate::generate::utils::doc_lines_from_annotations;
use serde_json::json;
use xidl_parser::hir;

pub(super) struct InterfaceMethodRenderer {
    type_policy: RustTypePolicy,
}

impl InterfaceMethodRenderer {
    pub(super) fn new() -> Self {
        Self {
            type_policy: RustTypePolicy,
        }
    }

    pub(super) fn render_op(&self, op: &hir::OpDcl) -> serde_json::Value {
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
            rust_passthrough_attrs_from_annotations(&op.annotations),
        )
    }

    pub(super) fn render_attr(&self, attr: &hir::AttrDcl) -> Vec<serde_json::Value> {
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
                        rust_passthrough_attrs_from_annotations(&attr.annotations),
                    )
                })
                .collect(),
            hir::AttrDclInner::AttrSpec(spec) => {
                let mut out = Vec::new();
                match &spec.declarator {
                    hir::AttrDeclarator::SimpleDeclarator(list) => {
                        for decl in list {
                            let name = crate::generate::rust::util::rust_ident(&decl.0);
                            out.extend(self.render_accessor_methods(
                                &name,
                                &spec.ty,
                                &doc,
                                rust_passthrough_attrs_from_annotations(&attr.annotations),
                            ));
                        }
                    }
                    hir::AttrDeclarator::WithRaises { declarator, .. } => {
                        let name = crate::generate::rust::util::rust_ident(&declarator.0);
                        out.extend(self.render_accessor_methods(
                            &name,
                            &spec.ty,
                            &doc,
                            rust_passthrough_attrs_from_annotations(&attr.annotations),
                        ));
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
            hir::ReadonlyAttrDeclarator::RaisesExpr(_) => Vec::new(),
        }
    }

    fn render_accessor_methods(
        &self,
        name: &str,
        ty: &hir::TypeSpec,
        doc: &[String],
        rust_attrs: Vec<String>,
    ) -> Vec<serde_json::Value> {
        let ret = self.type_policy.return_type(ty);
        let param = self.type_policy.param_type(ty, None);
        vec![
            self.method_json(name.to_string(), ret, Vec::new(), doc, rust_attrs.clone()),
            self.method_json(
                format!("set_{name}"),
                "()".to_string(),
                vec![format!("value: {param}")],
                doc,
                rust_attrs,
            ),
        ]
    }

    fn method_json(
        &self,
        name: String,
        ret: String,
        params: Vec<String>,
        doc: &[String],
        rust_attrs: Vec<String>,
    ) -> serde_json::Value {
        json!({
            "ret": ret,
            "name": name,
            "params": params,
            "doc": doc,
            "rust_attrs": rust_attrs,
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
