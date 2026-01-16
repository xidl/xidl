use crate::error::IdlcResult;
use crate::generate::cpp::util::member_json;
use crate::generate::cpp::{CppRender, CppRenderOutput, CppRenderer};
use serde_json::json;
use xidl_parser::hir;

impl CppRender for hir::ExceptDcl {
    fn render(&self, renderer: &CppRenderer) -> IdlcResult<CppRenderOutput> {
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
            "ident": &self.ident,
            "members": members,
        });
        let rendered = renderer.render_template("exception.h.j2", &ctx)?;
        Ok(CppRenderOutput::default().push_header(rendered))
    }
}
