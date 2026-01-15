use crate::error::IdlcResult;
use crate::generate::c::xcdr::{kind_json, type_kind_from_c};
use crate::generate::c::{CRender, CRenderOutput, CRenderer};
use serde_json::json;
use xidl_parser::hir;

impl CRender for hir::BitmaskDcl {
    fn render(&self, renderer: &CRenderer) -> IdlcResult<CRenderOutput> {
        let values: Vec<String> = self.value.iter().map(|value| value.ident.clone()).collect();
        let all_mask = if values.is_empty() {
            "0".to_string()
        } else {
            values.join(" | ")
        };
        let kind = type_kind_from_c("uint32_t");
        let xcdr_fields = vec![json!({
            "expr": "(*self)",
            "dims": Vec::<String>::new(),
            "kind": kind_json(&kind),
        })];
        let ctx = json!({
            "ident": &self.ident,
            "values": values,
            "all_mask": all_mask,
            "xcdr_fields": xcdr_fields,
        });
        let header = renderer.render_template("bitmask.h.j2", &ctx)?;
        let source = renderer.render_template("bitmask.c.j2", &ctx)?;
        let xcdr_header = renderer.render_template("bitmask_xcdr.h.j2", &ctx)?;
        let xcdr_source = renderer.render_template("bitmask_xcdr.c.j2", &ctx)?;
        Ok(CRenderOutput::default()
            .push_header(header)
            .push_source(source)
            .push_xcdr_header(xcdr_header)
            .push_xcdr_source(xcdr_source))
    }
}
