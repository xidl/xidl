use crate::error::IdlcResult;
use crate::generate::rust::util::{
    constr_type_scoped_name, declarator_name, render_const, rust_derive_info_with_extra,
    rust_scoped_name, rust_switch_type, serialize_kind_name, type_with_decl,
};
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use crate::generate::utils::doc_lines_from_annotations;
use itertools::Itertools;
use serde_json::json;
use std::collections::BTreeSet;
use xidl_parser::hir;

impl RustRender for hir::UnionForwardDcl {
    fn render(&self, _renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        Ok(RustRenderOutput::empty())
    }
}

impl RustRender for hir::UnionDef {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        render_union_with_config(self, renderer, &hir::SerializeConfig::default(), &[])
    }
}

pub(crate) fn render_union_with_config(
    def: &hir::UnionDef,
    renderer: &RustRenderer,
    config: &hir::SerializeConfig,
    module_path: &[String],
) -> IdlcResult<RustRenderOutput> {
    let mut fields = Vec::new();
    let mut seen = BTreeSet::new();
    for case in &def.case {
        let name = crate::generate::rust::util::rust_ident(&declarator_name(&case.element.value));
        if seen.insert(name.clone()) {
            let doc = doc_lines_from_annotations(&case.element.annotations);
            fields.push((name, &case.element.ty, &case.element.value, doc));
        }
    }

    let members = fields
        .into_iter()
        .map(|(name, ty, decl, doc)| {
            let ty = match ty {
                hir::ElementSpecTy::TypeSpec(spec) => type_with_decl(spec, decl),
                hir::ElementSpecTy::ConstrTypeDcl(constr) => {
                    rust_scoped_name(&constr_type_scoped_name(constr))
                }
            };
            json!({ "name": name, "ty": ty, "doc": doc })
        })
        .collect::<Vec<_>>();

    let cases = def
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
            let field_ty = match &case.element.ty {
                hir::ElementSpecTy::TypeSpec(spec) => type_with_decl(spec, &case.element.value),
                hir::ElementSpecTy::ConstrTypeDcl(constr) => {
                    rust_scoped_name(&constr_type_scoped_name(constr))
                }
            };
            json!({
                "labels": labels,
                "pattern": pattern,
                "type": field_ty,
                "is_default": is_default,
                "member": member,
                "field_id": field_id,
            })
        })
        .collect::<Vec<_>>();
    let has_default = cases
        .iter()
        .any(|case| case["is_default"].as_bool() == Some(true));

    let serialize_kind = serialize_kind_name(def.serialize_kind(config));
    let derive = rust_derive_info_with_extra(&def.annotations, &def.annotations);
    let ctx = renderer.enrich_scoped_ctx(
        json!({
            "ident": crate::generate::rust::util::rust_ident(&def.ident),
            "switch_ty": rust_switch_type(&def.switch_type_spec),
            "members": members,
            "cases": cases,
            "has_default": has_default,
            "serialize_kind": serialize_kind,
            "has_serde_serialize": derive.has_serde_serialize,
            "has_serde_deserialize": derive.has_serde_deserialize,
            "derive": derive.non_serde.into_iter().collect_vec(),
        }),
        &doc_lines_from_annotations(&def.annotations),
        module_path,
    );
    renderer.render_source_template("union.rs.j2", &ctx)
}
