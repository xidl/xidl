use crate::error::IdlcResult;
use crate::generate::cpp::{CppRender, CppRenderOutput, CppRenderer};
use serde_json::json;
use xidl_parser::hir;

impl CppRender for hir::EnumDcl {
    fn render(&self, renderer: &CppRenderer) -> IdlcResult<CppRenderOutput> {
        let members = self
            .member
            .iter()
            .map(|member| member.ident.clone())
            .collect::<Vec<_>>();
        let ctx = json!({ "ident": &self.ident, "members": members });
        let rendered = renderer.render_template("enum.h.j2", &ctx)?;
        Ok(CppRenderOutput::default().push_header(rendered))
    }
}
