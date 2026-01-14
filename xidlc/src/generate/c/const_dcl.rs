use crate::error::IdlcResult;
use crate::generate::c::util::{c_const_type, c_literal, c_scoped_name};
use crate::generate::c::{CRender, CRenderer};
use crate::generate::render_const_expr;
use serde_json::json;
use xidl_parser::hir;

impl CRender for hir::ConstDcl {
    fn render(&self, renderer: &CRenderer) -> IdlcResult<Vec<String>> {
        let ctx = json!({
            "ident": &self.ident,
            "ty": c_const_type(&self.ty),
            "value": render_const_expr(&self.value, &c_scoped_name, &c_literal),
        });
        let rendered = renderer.render_template("const.h.j2", &ctx)?;
        Ok(vec![rendered])
    }
}
