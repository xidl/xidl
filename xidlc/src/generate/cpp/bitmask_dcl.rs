use crate::error::IdlcResult;
use crate::generate::cpp::{CppRender, CppRenderOutput, CppRenderer};
use serde_json::json;
use xidl_parser::hir;

impl CppRender for hir::BitmaskDcl {
    fn render(&self, renderer: &CppRenderer) -> IdlcResult<CppRenderOutput> {
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
        let rendered = renderer.render_template("bitmask.h.j2", &ctx)?;
        Ok(CppRenderOutput::default().push_header(rendered))
    }
}
