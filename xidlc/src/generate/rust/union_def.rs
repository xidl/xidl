use crate::error::IdlcResult;
use crate::generate::rust::util::{
    declarator_name, render_const, rust_scoped_name, rust_switch_type, serialize_kind_name,
    type_with_decl,
};
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use itertools::Itertools;
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
        render_union_with_config(self, renderer, &hir::SerializeConfig::default(), &[], &[])
    }
}

pub(crate) fn render_union_with_config(
    def: &hir::UnionDef,
    renderer: &RustRenderer,
    config: &hir::SerializeConfig,
    module_path: &[String],
    annotations: &[hir::Annotation],
) -> IdlcResult<RustRenderOutput> {
    let mut fields = Vec::new();
    let mut seen = BTreeSet::new();
    for case in &def.case {
        let name = crate::generate::rust::util::rust_ident(&declarator_name(&case.element.value));
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
                    rust_scoped_name(&constr_type_name(constr))
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
    let derive = crate::generate::rust::util::rust_derives_from_annotations_with_extra(
        &def.annotations,
        annotations,
    );
    let module_path = module_path.to_vec();

    let ctx = json!({
        "ident": crate::generate::rust::util::rust_ident(&def.ident),
        "switch_ty": rust_switch_type(&def.switch_type_spec),
        "members": members,
        "cases": cases,
        "has_default": has_default,
        "serialize_kind": serialize_kind,
        "has_serde_serialize": derive.iter().find(|v|*v=="::serde::Serialize"),
        "has_serde_deserialize": derive.iter().find(|v|*v=="::serde::Deserialize"),
        "derive": derive.into_iter().filter(|v| v != "::serde::Serialize" && v!="::serde::Deserialize").collect_vec(),
        "module_path": module_path,
        "typeobject_path": renderer.typeobject_path(),
    });
    let rendered = renderer.render_template("union.rs.j2", &ctx)?;
    Ok(RustRenderOutput::default().push_source(rendered))
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
