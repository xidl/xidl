use crate::error::IdlcResult;
use crate::generate::rust::util::{member_json, rust_scoped_name, serialize_kind_name};
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use serde_json::json;
use xidl_parser::hir;

impl RustRender for hir::StructForwardDcl {
    fn render(&self, _renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        Ok(RustRenderOutput::default())
    }
}

impl RustRender for hir::StructDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        render_struct_with_config(self, renderer, &hir::SerializeConfig::default())
    }
}

pub(crate) fn render_struct_with_config(
    def: &hir::StructDcl,
    renderer: &RustRenderer,
    config: &hir::SerializeConfig,
) -> IdlcResult<RustRenderOutput> {
    let parent = def.parent.first().map(rust_scoped_name);
    let members = def
        .member
        .iter()
        .flat_map(|member| {
            member
                .ident
                .iter()
                .map(|decl| member_json(&member.ty, decl))
        })
        .collect::<Vec<_>>();
    let serialize_kind = serialize_kind_name(def.serialize_kind(config));
    let ctx = json!({
        "ident": crate::generate::rust::util::rust_ident(&def.ident),
        "parent": parent,
        "members": members,
        "serialize_kind": serialize_kind,
    });
    let rendered = renderer.render_template("struct.rs.j2", &ctx)?;
    Ok(RustRenderOutput::default().push_source(rendered))
}
