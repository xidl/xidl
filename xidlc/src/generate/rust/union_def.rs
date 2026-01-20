use crate::error::IdlcResult;
use crate::generate::rust::util::{
    declarator_name, render_const, rust_scoped_name, rust_switch_type, type_with_decl,
};
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use serde_json::json;
use std::collections::BTreeSet;
use xidl_parser::hir;

impl RustRender for hir::UnionForwardDcl {
    fn render(&self, _renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        Ok(RustRenderOutput::default())
    }
}

impl RustRender for hir::UnionDef {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        let mut fields = Vec::new();
        let mut seen = BTreeSet::new();
        for case in &self.case {
            let name =
                crate::generate::rust::util::rust_ident(&declarator_name(&case.element.value));
            if seen.insert(name.clone()) {
                fields.push((name, &case.element.ty, &case.element.value));
            }
        }

        let members = fields
            .into_iter()
            .map(|(name, ty, decl)| {
                let ty = match ty {
                    hir::ElementSpecTy::TypeSpec(spec) => type_with_decl(spec, decl),
                    hir::ElementSpecTy::ConstrTypeDcl(constr) => {
                        rust_scoped_name(&constr_type_name(constr))
                    }
                };
                json!({ "name": name, "ty": ty })
            })
            .collect::<Vec<_>>();

        let cases = self
            .case
            .iter()
            .map(|case| {
                let labels = case
                    .label
                    .iter()
                    .map(|label| match label {
                        hir::CaseLabel::Value(expr) => render_const(expr),
                        hir::CaseLabel::Default => "default".to_string(),
                    })
                    .collect::<Vec<_>>();
                let is_default = labels.iter().any(|label| label == "default");
                let pattern = if is_default {
                    "_".to_string()
                } else {
                    labels.join(" | ")
                };
                let member =
                    crate::generate::rust::util::rust_ident(&declarator_name(&case.element.value));
                let field_id = case
                    .element
                    .field_id
                    .map(|value| format!("{value}u32"))
                    .unwrap_or_else(|| "1u32".to_string());
                json!({
                    "labels": labels,
                    "pattern": pattern,
                    "is_default": is_default,
                    "member": member,
                    "field_id": field_id,
                })
            })
            .collect::<Vec<_>>();
        let has_default = cases
            .iter()
            .any(|case| case["is_default"].as_bool() == Some(true));

        let ctx = json!({
            "ident": crate::generate::rust::util::rust_ident(&self.ident),
            "switch_ty": rust_switch_type(&self.switch_type_spec),
            "members": members,
            "cases": cases,
            "has_default": has_default,
        });
        let rendered = renderer.render_template("union.rs.j2", &ctx)?;
        Ok(RustRenderOutput::default().push_source(rendered))
    }
}

fn constr_type_name(constr: &hir::ConstrTypeDcl) -> hir::ScopedName {
    let name = match constr {
        hir::ConstrTypeDcl::StructForwardDcl(def) => def.ident.clone(),
        hir::ConstrTypeDcl::StructDcl(def) => def.ident.clone(),
        hir::ConstrTypeDcl::EnumDcl(def) => def.ident.clone(),
        hir::ConstrTypeDcl::UnionForwardDcl(def) => def.ident.clone(),
        hir::ConstrTypeDcl::UnionDef(def) => def.ident.clone(),
        hir::ConstrTypeDcl::BitsetDcl(def) => def.ident.clone(),
        hir::ConstrTypeDcl::BitmaskDcl(def) => def.ident.clone(),
    };
    hir::ScopedName {
        name: vec![name],
        is_root: false,
    }
}
