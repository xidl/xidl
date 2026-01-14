use crate::error::IdlcResult;
use crate::generate::c::{CRender, CRenderOutput, CRenderer};
use serde_json::json;
use xidl_parser::hir;

impl CRender for hir::BitmaskDcl {
    fn render(&self, renderer: &CRenderer) -> IdlcResult<CRenderOutput> {
        let all_mask = if self.value.is_empty() {
            "0".to_string()
        } else {
            self.value.join(" | ")
        };
        let ctx = json!({
            "ident": &self.ident,
            "values": self.value.clone(),
            "all_mask": all_mask,
        });
        let header = renderer.render_template("bitmask.h.j2", &ctx)?;
        let source = renderer.render_template("bitmask.c.j2", &ctx)?;
        Ok(CRenderOutput::default()
            .push_header(header)
            .push_source(source))
    }
}
