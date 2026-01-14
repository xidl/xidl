use crate::error::IdlcResult;
use crate::generate::c::{CRender, CRenderOutput, CRenderer};
use serde_json::json;
use xidl_parser::hir;

impl CRender for hir::EnumDcl {
    fn render(&self, renderer: &CRenderer) -> IdlcResult<CRenderOutput> {
        let ctx = json!({
            "ident": &self.ident,
            "variants": self
                .member
                .iter()
                .collect::<Vec<_>>(),
        });
        let header = renderer.render_template("enum.h.j2", &ctx)?;
        let source = renderer.render_template("enum.c.j2", &ctx)?;
        Ok(CRenderOutput::default()
            .push_header(header)
            .push_source(source))
    }
}
