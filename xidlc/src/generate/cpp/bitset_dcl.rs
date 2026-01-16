use crate::error::IdlcResult;
use crate::generate::cpp::util::{bitfield_type, cpp_scoped_name, render_const};
use crate::generate::cpp::{CppRender, CppRenderOutput, CppRenderer};
use serde_json::json;
use xidl_parser::hir;

impl CppRender for hir::BitsetDcl {
    fn render(&self, renderer: &CppRenderer) -> IdlcResult<CppRenderOutput> {
        let parent = self.parent.as_ref().map(cpp_scoped_name);
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
                    "name": field.ident,
                    "width": width,
                })
            })
            .collect::<Vec<_>>();
        let ctx = json!({
            "ident": &self.ident,
            "parent": parent,
            "fields": fields,
        });
        let rendered = renderer.render_template("bitset.h.j2", &ctx)?;
        Ok(CppRenderOutput::default().push_header(rendered))
    }
}
