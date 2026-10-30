use super::util::http_union_case_ident;
use crate::error::IdlcResult;
use crate::generate::rust::util::{
    constr_type_scoped_name, declarator_name, render_const, rust_deprecated_attr,
    rust_derive_info_with_extra, rust_ident, rust_passthrough_attrs_from_annotations,
    rust_scoped_name, rust_switch_type,
};
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use crate::generate::utils::doc_lines_from_annotations;
use crate::generate::utils::union_wire::UnionWire;
use convert_case::{Case as ConvertCase, Casing};
use serde_json::json;
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

/// `@http` unions render as a plain untagged enum: HTTP codegen dispatches
/// per case on the negotiated media type, so no wire-level tag exists.
fn render_http_union(def: &hir::UnionDef, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
    let cases = def
        .case
        .iter()
        .map(|case| {
            let [hir::CaseLabel::Value(hir::ConstExpr::ScopedName(name))] = case.label.as_slice()
            else {
                return Err(crate::error::IdlcError::rpc(
                    "HTTP union case requires one ContentType member label",
                ));
            };
            let case_ident = name.name.last().ok_or_else(|| {
                crate::error::IdlcError::rpc("HTTP union case has an empty ContentType label")
            })?;
            let ty = match &case.element.ty {
                hir::ElementSpecTy::TypeSpec(ty) => {
                    renderer.declared_type(ty, &case.element.value)?
                }
                hir::ElementSpecTy::ConstrTypeDcl(_) => {
                    return Err(crate::error::IdlcError::rpc(
                        "HTTP union payload requires a named type",
                    ));
                }
            };
            Ok(json!({
                "case": http_union_case_ident(case_ident),
                "ty": ty,
                "doc": doc_lines_from_annotations(&case.element.annotations),
            }))
        })
        .collect::<IdlcResult<Vec<_>>>()?;
    let ctx = renderer.enrich_ctx(
        renderer.with_ident(
            json!({
                "cases": cases,
                "rust_attrs": rust_attrs_from(def)?,
            }),
            &def.ident,
        ),
        &doc_lines_from_annotations(&def.annotations),
    );
    renderer.render_source_template("http_union.rs.j2", &ctx)
}

pub(crate) fn render_union_with_config(
    def: &hir::UnionDef,
    renderer: &RustRenderer,
    module_path: &[String],
) -> IdlcResult<RustRenderOutput> {
    if crate::generate::utils::has_annotation(&def.annotations, "http") {
        return render_http_union(def, renderer);
    }
    let allow_deprecated = def
        .case
        .iter()
        .any(|case| case.element.references_deprecated)
        || rust_deprecated_attr(&[&def.annotations])?.is_some();
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
            let member = rust_ident(&declarator_name(&case.element.value));
            let helper_variant = member.to_case(ConvertCase::Pascal);
            let mut field_ty = match &case.element.ty {
                hir::ElementSpecTy::TypeSpec(spec) => {
                    renderer.declared_type(spec, &case.element.value)?
                }
                hir::ElementSpecTy::ConstrTypeDcl(constr) => {
                    rust_scoped_name(&constr_type_scoped_name(constr))
                }
            };
            if case.element.recursive {
                field_ty = format!("Box<{field_ty}>");
            }
            let serde_arms = case
                .label
                .iter()
                .zip(&labels)
                .map(|(wire, label)| {
                    let is_default = matches!(wire, hir::CaseLabel::Default);
                    json!({
                        "match_expr": label,
                        "tag_literal": format!("{:?}", UnionWire::label(wire)),
                        "tag_expr": label,
                        "is_default": is_default,
                    })
                })
                .collect::<Vec<_>>();
            Ok(json!({
                "case_index": case_index,
                "constructor_tag": labels.first().ok_or_else(|| crate::error::IdlcError::rpc(
                    format!("union '{}' has a case without a label", def.ident)
                ))?,
                "constructor_default": matches!(case.label.first(), Some(hir::CaseLabel::Default)),
                "labels": labels,
                "serde_arms": serde_arms,
                "type": field_ty,
                "is_default": is_default,
                "member": member,
                "helper_variant": helper_variant,
            }))
        })
        .collect::<IdlcResult<Vec<_>>>()?;

    let default_case = cases
        .iter()
        .find(|case| case["is_default"].as_bool() == Some(true))
        .or_else(|| cases.first())
        .map(|case| json!({ "member": case["member"].clone() }));

    let derive = rust_derive_info_with_extra(&def.annotations, &def.annotations);

    let ctx = renderer.enrich_scoped_ctx(
        json!({
            "ident": crate::generate::rust::util::rust_ident(&def.ident),
            "switch_ty": rust_switch_type(&def.switch_type_spec),
            "cases": cases,
            "default_case": default_case,
            "has_serde_serialize": derive.has_serde_serialize,
            "has_serde_deserialize": derive.has_serde_deserialize,
            "derives": derive.non_serde,
            "rust_attrs": rust_attrs_from(def)?,
            "deprecated": allow_deprecated,
        }),
        &doc_lines_from_annotations(&def.annotations),
        module_path,
    );
    renderer.render_source_template("union.rs.j2", &ctx)
}

/// Passthrough attributes plus the `deprecated` attribute when declared.
fn rust_attrs_from(def: &hir::UnionDef) -> IdlcResult<Vec<String>> {
    let mut attrs = rust_passthrough_attrs_from_annotations(&def.annotations);
    attrs.extend(rust_deprecated_attr(&[&def.annotations])?);
    Ok(attrs)
}
