use crate::error::IdlcResult;
use crate::generate::rust::util::{render_const, rust_const_type};
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use serde_json::json;
use xidl_parser::hir;

impl RustRender for hir::ConstDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        let ty = rust_const_type(&self.ty);
        let value = render_const(&self.value);
        let ctx = json!({
            "ty": ty,
            "ident": &self.ident,
            "value": value,
        });
        let rendered = renderer.render_template("const.rs.j2", &ctx)?;
        Ok(RustRenderOutput::default().push_source(rendered))
    }
}
