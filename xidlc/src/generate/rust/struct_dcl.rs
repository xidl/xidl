use crate::error::IdlcResult;
use crate::generate::rust::util::{
    declarator_name, rust_derive_info_with_extra, rust_scoped_name, serde_rename_from_annotations,
    serialize_kind_name, type_with_decl,
};
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use crate::generate::utils::doc_lines_from_annotations;
use serde_json::json;
use xidl_parser::hir;

impl RustRender for hir::StructForwardDcl {
    fn render(&self, _renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        Ok(RustRenderOutput::empty())
    }
}

impl RustRender for hir::StructDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        render_struct_with_config(self, renderer, &hir::SerializeConfig::default(), &[])
    }
}

pub(crate) fn render_struct_with_config(
    def: &hir::StructDcl,
    renderer: &RustRenderer,
    config: &hir::SerializeConfig,
    module_path: &[String],
) -> IdlcResult<RustRenderOutput> {
    let parent = def.parent.first().map(rust_scoped_name);
    let members = def
        .member
        .iter()
        .flat_map(|member| {
            let field_id = member.field_id.map(|value| format!("{value}u32"));
            let optional = member.is_optional();
            let rename = serde_rename_from_annotations(&member.annotations);
            let doc = doc_lines_from_annotations(&member.annotations);
            member.ident.iter().map(move |decl| {
                let name = crate::generate::rust::util::rust_ident(&declarator_name(decl));
                let mut ty = type_with_decl(&member.ty, decl);
                if optional {
                    ty = format!("Option<{ty}>");
                }
                json!({
                    "ty": ty,
                    "name": name,
                    "serde_rename": rename,
                    "field_id": field_id.clone(),
                    "optional": optional,
                    "doc": doc
                })
            })
        })
        .collect::<Vec<_>>();
    let serialize_kind = serialize_kind_name(def.serialize_kind(config));
    let derive = rust_derive_info_with_extra(&def.annotations, &def.annotations);
    let ctx = renderer.enrich_scoped_ctx(
        json!({
            "ident": crate::generate::rust::util::rust_ident(&def.ident),
            "parent": parent,
            "members": members,
            "serialize_kind": serialize_kind,
            "derive": derive.all,
            "enable_serde_attrs": derive.enable_serde_attrs(),
        }),
        &doc_lines_from_annotations(&def.annotations),
        module_path,
    );
    renderer.render_source_template("struct.rs.j2", &ctx)
}
