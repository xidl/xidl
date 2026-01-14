use crate::error::IdlcResult;
use crate::generate::c::util::{bitfield_type, c_scoped_name_hir};
use crate::generate::c::{CRender, CRenderer};
use serde_json::json;
use xidl_parser::hir;

impl CRender for hir::BitsetDcl {
    fn render(&self, renderer: &CRenderer) -> IdlcResult<Vec<String>> {
        let fields = self
            .field
            .iter()
            .map(|field| {
                let ty = field
                    .ty
                    .as_ref()
                    .map(bitfield_type)
                    .unwrap_or_else(|| "uint32_t".to_string());
                let pos = crate::generate::render_const_expr(
                    &field.pos.0,
                    &crate::generate::c::util::c_scoped_name,
                    &crate::generate::c::util::c_literal,
                );
                json!({
                    "ident": &field.ident,
                    "ty": ty,
                    "pos": pos,
                })
            })
            .collect::<Vec<_>>();

        let ctx = json!({
            "ident": &self.ident,
            "parent": self.parent.as_ref().map(c_scoped_name_hir),
            "fields": fields,
        });

        let rendered = renderer.render_template("bitset.h.j2", &ctx)?;
        Ok(vec![rendered])
    }
}
