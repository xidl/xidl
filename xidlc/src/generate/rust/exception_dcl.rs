use crate::error::IdlcResult;
use crate::generate::rust::util::member_json;
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use serde_json::json;
use xidl_parser::hir;

impl RustRender for hir::ExceptDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
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
            "members": members,
        });
        let rendered = renderer.render_template("exception.rs.j2", &ctx)?;
        Ok(RustRenderOutput::default().push_source(rendered))
    }
}
