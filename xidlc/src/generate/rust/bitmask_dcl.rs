use crate::error::IdlcResult;
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use serde_json::json;
use xidl_parser::hir;

impl RustRender for hir::BitmaskDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        let values = self
            .value
            .iter()
            .enumerate()
            .map(|(idx, value)| {
                json!({
                    "name": value.ident,
                    "index": idx,
                })
            })
            .collect::<Vec<_>>();
        let ctx = json!({
            "ident": &self.ident,
            "values": values,
        });
        let rendered = renderer.render_template("bitmask.rs.j2", &ctx)?;
        Ok(RustRenderOutput::default().push_source(rendered))
    }
}
