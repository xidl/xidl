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
                    "name": crate::generate::rust::util::rust_ident(&value.ident),
                    "index": idx,
                })
            })
            .collect::<Vec<_>>();
        let derive = crate::generate::rust::util::rust_derives_from_annotations_with_extra(
            &self.annotations,
            &self.annotations,
        );
        let ctx = json!({
            "ident": crate::generate::rust::util::rust_ident(&self.ident),
            "values": values,
            "derive": derive,
            "typeobject_path": renderer.typeobject_path(),
        });
        let rendered = renderer.render_template("bitmask.rs.j2", &ctx)?;
        Ok(RustRenderOutput::default().push_source(rendered))
    }
}
