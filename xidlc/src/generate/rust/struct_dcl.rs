use crate::error::IdlcResult;
use crate::generate::rust::util::{member_json, rust_scoped_name};
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
        let parent = self.parent.first().map(rust_scoped_name);
        let members = self
            .member
            .iter()
            .flat_map(|member| {
                member
                    .ident
                    .iter()
                    .map(|decl| member_json(&member.ty, decl))
            })
            .collect::<Vec<_>>();
        let ctx = json!({
            "ident": crate::generate::rust::util::rust_ident(&self.ident),
            "parent": parent,
            "members": members,
        });
        let rendered = renderer.render_template("struct.rs.j2", &ctx)?;
        Ok(RustRenderOutput::default().push_source(rendered))
    }
}
