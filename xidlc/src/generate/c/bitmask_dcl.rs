use crate::error::IdlcResult;
use crate::generate::c::{CRender, CRenderer};
use serde_json::json;
use xidl_parser::hir;

impl CRender for hir::BitmaskDcl {
    fn render(&self, renderer: &CRenderer) -> IdlcResult<Vec<String>> {
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
        let rendered = renderer.render_template("bitmask.h.j2", &ctx)?;
        Ok(vec![rendered])
    }
}
