use crate::error::IdlcResult;
use crate::generate::rust::util::{
    constr_type_scoped_name, declarator_name, render_const, rust_derive_info_with_extra,
    rust_ident, rust_passthrough_attrs_from_annotations, rust_scoped_name, rust_switch_type,
    type_with_decl,
};
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use crate::generate::utils::doc_lines_from_annotations;
use convert_case::{Case as ConvertCase, Casing};
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
        render_union_with_config(self, renderer, &[])
    }
}

pub(crate) fn render_union_with_config(
    def: &hir::UnionDef,
    renderer: &RustRenderer,
    module_path: &[String],
) -> IdlcResult<RustRenderOutput> {
    let mut fields = Vec::new();
    let mut seen = BTreeSet::new();
    for case in &def.case {
        let name = crate::generate::rust::util::rust_ident(&declarator_name(&case.element.value));
        if seen.insert(name.clone()) {
            let doc = doc_lines_from_annotations(&case.element.annotations);
            let rust_attrs = rust_passthrough_attrs_from_annotations(&case.element.annotations);
            fields.push((
                name,
                &case.element.ty,
                &case.element.value,
                doc,
                rust_attrs,
                case.element.recursive,
            ));
        }
    }

    let members = fields
        .into_iter()
        .map(|(name, ty, decl, doc, rust_attrs, recursive)| {
            let mut ty = match ty {
                hir::ElementSpecTy::TypeSpec(spec) => type_with_decl(spec, decl),
                hir::ElementSpecTy::ConstrTypeDcl(constr) => {
                    rust_scoped_name(&constr_type_scoped_name(constr))
                }
            };
            if recursive {
                ty = format!("Box<{ty}>");
            }
            json!({ "name": name, "ty": ty, "doc": doc, "rust_attrs": rust_attrs })
        })
        .collect::<Vec<_>>();

    let cases = def
        .case
        .iter()
        .enumerate()
        .map(|(case_index, case)| {
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
            let member = rust_ident(&declarator_name(&case.element.value));
            let helper_variant = member.to_case(ConvertCase::Pascal);
            let field_id = case
                .element
                .field_id
                .map(|value| format!("{value}u32"))
                .unwrap_or_else(|| "1u32".to_string());
            let mut field_ty = match &case.element.ty {
                hir::ElementSpecTy::TypeSpec(spec) => type_with_decl(spec, &case.element.value),
                hir::ElementSpecTy::ConstrTypeDcl(constr) => {
                    rust_scoped_name(&constr_type_scoped_name(constr))
                }
            };
            if case.element.recursive {
                field_ty = format!("Box<{field_ty}>");
            }
            let serde_arms = if is_default {
                vec![json!({
                    "match_expr": "_",
                    "tag": "default",
                    "tag_expr": format!("{}::default()", rust_switch_type(&def.switch_type_spec)),
                })]
            } else {
                labels
                    .iter()
                    .map(|label| {
                        json!({
                            "match_expr": label,
                            "tag": serde_tag_string(label),
                            "tag_expr": label,
                        })
                    })
                    .collect::<Vec<_>>()
            };
            json!({
                "case_index": case_index,
                "constructor_tag": if is_default {
                    format!("{}::default()", rust_switch_type(&def.switch_type_spec))
                } else {
                    labels
                        .first()
                        .expect("union case label")
                        .to_string()
                },
                "labels": labels,
                "serde_arms": serde_arms,
                "pattern": pattern,
                "type": field_ty,
                "is_default": is_default,
                "member": member,
                "helper_variant": helper_variant,
                "field_id": field_id,
            })
        })
        .collect::<Vec<_>>();
    let has_default = cases
        .iter()
        .any(|case| case["is_default"].as_bool() == Some(true));
    let default_case = cases
        .iter()
        .find(|case| case["is_default"].as_bool() == Some(true))
        .or_else(|| cases.first())
        .map(|case| {
            let tag = if case["is_default"].as_bool() == Some(true) {
                format!("{}::default()", rust_switch_type(&def.switch_type_spec))
            } else {
                case["labels"][0]
                    .as_str()
                    .expect("union case label")
                    .to_string()
            };
            json!({
                "member": case["member"].clone(),
                "type": case["type"].clone(),
                "tag": tag,
            })
        });

    let derive = rust_derive_info_with_extra(&def.annotations, &def.annotations);
    let ctx = renderer.enrich_scoped_ctx(
        json!({
            "ident": crate::generate::rust::util::rust_ident(&def.ident),
            "switch_ty": rust_switch_type(&def.switch_type_spec),
            "members": members,
            "cases": cases,
            "has_default": has_default,
            "default_case": default_case,
            "has_serde_serialize": derive.has_serde_serialize,
            "has_serde_deserialize": derive.has_serde_deserialize,
            "derive": derive.non_serde.into_iter().collect_vec(),
            "rust_attrs": rust_passthrough_attrs_from_annotations(&def.annotations),
        }),
        &doc_lines_from_annotations(&def.annotations),
        module_path,
    );
    renderer.render_source_template("union.rs.j2", &ctx)
}

fn serde_tag_string(label: &str) -> String {
    label.rsplit("::").next().unwrap_or(label).to_string()
}
