use crate::error::IdlcResult;
use crate::generate::c::{CRender, CRenderer};
use serde_json::json;
use xidl_parser::hir;

impl CRender for hir::BitmaskDcl {
    fn render(&self, renderer: &CRenderer) -> IdlcResult<Vec<String>> {
        let ctx = json!({
            "ident": &self.ident,
            "values": self.value.clone(),
        });
        let rendered = renderer.render_template("bitmask.h.j2", &ctx)?;
        Ok(vec![rendered])
    }
}
