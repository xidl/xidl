use crate::error::IdlcResult;
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use serde_json::json;
use xidl_parser::hir;

impl RustRender for hir::EnumDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        let members = self
            .member
            .iter()
            .map(|member| crate::generate::rust::util::rust_ident(&member.ident))
            .collect::<Vec<_>>();
        let ctx = json!({
            "ident": crate::generate::rust::util::rust_ident(&self.ident),
            "members": members
        });
        let rendered = renderer.render_template("enum.rs.j2", &ctx)?;
        Ok(RustRenderOutput::default().push_source(rendered))
    }
}
