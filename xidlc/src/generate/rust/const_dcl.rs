use crate::error::IdlcResult;
use crate::generate::rust::util::{render_const, rust_const_type};
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use serde_json::json;
use xidl_parser::hir;

impl RustRender for hir::ConstDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        let ty = rust_const_type(&self.ty);
        let value = render_const(&self.value);
        let ctx = renderer.with_ident(
            json!({
                "ty": ty,
                "value": value,
            }),
            &self.ident,
        );
        renderer.render_source_template("const.rs.j2", &ctx)
    }
}
