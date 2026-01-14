use crate::error::IdlcResult;
use crate::generate::c::util::field_for_member;
use crate::generate::c::{CRender, CRenderOutput, CRenderer};
use serde_json::json;
use xidl_parser::hir;

impl CRender for hir::StructDcl {
    fn render(&self, renderer: &CRenderer) -> IdlcResult<CRenderOutput> {
        let fields = self
            .member
            .iter()
            .flat_map(|member| {
                member
                    .ident
                    .iter()
                    .map(|decl| field_for_member(member, decl))
            })
            .collect::<Vec<_>>();

        let ctx = json!({
            "ident": &self.ident,
            "members": fields,
        });
        let header = renderer.render_template("struct.h.j2", &ctx)?;
        let source = renderer.render_template("struct.c.j2", &ctx)?;
        Ok(CRenderOutput::default()
            .push_header(header)
            .push_source(source))
    }
}
