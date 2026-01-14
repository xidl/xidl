use crate::error::IdlcResult;
use crate::generate::c::{CRender, CRenderer};
use serde_json::json;
use xidl_parser::hir;

impl CRender for hir::EnumDcl {
    fn render(&self, renderer: &CRenderer) -> IdlcResult<Vec<String>> {
        let ctx = json!({
            "ident": &self.ident,
            "variants": self
                .member
                .iter()
                .map(|member| member)
                .collect::<Vec<_>>(),
        });
        let rendered = renderer.render_template("enum.h.j2", &ctx)?;
        Ok(vec![rendered])
    }
}
