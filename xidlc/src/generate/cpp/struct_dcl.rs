use crate::error::IdlcResult;
use crate::generate::cpp::util::{cpp_scoped_name, member_json};
use crate::generate::cpp::{CppRender, CppRenderOutput, CppRenderer};
use serde_json::json;
use xidl_parser::hir;

impl CppRender for hir::StructForwardDcl {
    fn render(&self, renderer: &CppRenderer) -> IdlcResult<CppRenderOutput> {
        let ctx = json!({ "kind": "struct", "ident": &self.ident });
        let rendered = renderer.render_template("forward.h.j2", &ctx)?;
        Ok(CppRenderOutput::default().push_header(rendered))
    }
}

impl CppRender for hir::StructDcl {
    fn render(&self, renderer: &CppRenderer) -> IdlcResult<CppRenderOutput> {
        let parent = self.parent.first().map(cpp_scoped_name);
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
            "parent": parent,
            "members": members,
        });
        let rendered = renderer.render_template("struct.h.j2", &ctx)?;
        Ok(CppRenderOutput::default().push_header(rendered))
    }
}
