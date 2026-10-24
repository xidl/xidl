use crate::error::IdlcResult;
use crate::generate::rust::util::{
    declarator_name, is_skipped_from_annotations, rust_deprecated_attr,
    rust_derive_info_with_extra, rust_passthrough_attrs_from_annotations, rust_scoped_name,
    type_with_decl,
};
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use crate::generate::utils::{doc_lines_from_annotations, has_annotation};
use serde_json::json;
use xidl_parser::hir;

impl RustRender for hir::StructForwardDcl {
    fn render(&self, _renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        Ok(RustRenderOutput::empty())
    }
}

impl RustRender for hir::StructDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        render_struct_with_config(self, renderer, &[])
    }
}

pub(crate) fn render_struct_with_config(
    def: &hir::StructDcl,
    renderer: &RustRenderer,
    module_path: &[String],
) -> IdlcResult<RustRenderOutput> {
    let parent = def.parent.first().map(rust_scoped_name);
    let deprecated = def
        .member
        .iter()
        .map(|member| rust_deprecated_attr(&[&member.annotations]))
        .collect::<IdlcResult<Vec<_>>>()?;
    let members = def
        .member
        .iter()
        .zip(deprecated)
        .flat_map(|(member, deprecated)| {
            let field_id = member.field_id.map(|value| format!("{value}u32"));
            let optional = member.is_optional();
            let skip = is_skipped_from_annotations(&member.annotations);
            let flatten = has_annotation(&member.annotations, "flatten");
            let doc = doc_lines_from_annotations(&member.annotations);
            let mut rust_attrs = rust_passthrough_attrs_from_annotations(&member.annotations);
            rust_attrs.extend(deprecated);
            member.ident.iter().map(move |decl| {
                let raw_name = declarator_name(decl);
                let name = crate::generate::rust::util::rust_ident(&raw_name);
                let mut ty = type_with_decl(&member.ty, decl);
                if member.recursive {
                    ty = format!("Box<{ty}>");
                }
                if optional {
                    ty = format!("Option<{ty}>");
                }
                let wire_name =
                    hir::effective_wire_name(&raw_name, &member.annotations, &def.annotations);
                let rename = (wire_name != raw_name).then_some(wire_name);
                json!({
                    "ty": ty,
                    "name": name,
                    "serde_rename": rename,
                    "serde_skip": skip,
                    "serde_flatten": flatten,
                    "field_id": field_id.clone(),
                    "optional": optional,
                    "doc": doc,
                    "rust_attrs": rust_attrs,
                })
            })
        })
        .collect::<Vec<_>>();
    let derive = rust_derive_info_with_extra(&def.annotations, &def.annotations);
    // Mechanical references live inside the type's own declaration and
    // impls: field types, `#[default]` variants and derive expansions all
    // resolve to deprecated items, so the whole struct silences the lint
    // while consumer code outside stays warned.
    let allow_deprecated = def.member.iter().any(|member| member.references_deprecated)
        || rust_deprecated_attr(&[&def.annotations])?.is_some();
    let mut rust_attrs = rust_passthrough_attrs_from_annotations(&def.annotations);
    if allow_deprecated {
        rust_attrs.push("allow(deprecated)".to_string());
    }
    let ctx = renderer.enrich_scoped_ctx(
        json!({
            "ident": crate::generate::rust::util::rust_ident(&def.ident),
            "parent": parent,
            "members": members,
            "derive": derive.all,
            "enable_serde_attrs": derive.enable_serde_attrs(),
            "rust_attrs": rust_attrs,
            "deprecated": allow_deprecated,
        }),
        &doc_lines_from_annotations(&def.annotations),
        module_path,
    );
    renderer.render_source_template("struct.rs.j2", &ctx)
}
