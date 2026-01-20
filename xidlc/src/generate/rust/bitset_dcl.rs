use crate::error::IdlcResult;
use crate::generate::rust::util::{bitfield_type, render_const, rust_scoped_name};
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use serde_json::json;
use xidl_parser::hir;

impl RustRender for hir::BitsetDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        let parent = self.parent.as_ref().map(rust_scoped_name);
        let fields = self
            .field
            .iter()
            .map(|field| {
                let ty = field
                    .ty
                    .as_ref()
                    .map(bitfield_type)
                    .unwrap_or_else(|| "bool".to_string());
                let width = render_const(&field.pos.0);
                json!({
                    "ty": ty,
                    "name": crate::generate::rust::util::rust_ident(&field.ident),
                    "width": width,
                })
            })
            .collect::<Vec<_>>();
        let ctx = json!({
            "ident": crate::generate::rust::util::rust_ident(&self.ident),
            "parent": parent,
            "fields": fields,
        });
        let rendered = renderer.render_template("bitset.rs.j2", &ctx)?;
        Ok(RustRenderOutput::default().push_source(rendered))
    }
}
