use crate::error::IdlcResult;
use crate::generate::cpp::util::{cpp_const_type, render_const};
use crate::generate::cpp::{CppRender, CppRenderOutput, CppRenderer};
use serde_json::json;
use xidl_parser::hir;

impl CppRender for hir::ConstDcl {
    fn render(&self, renderer: &CppRenderer) -> IdlcResult<CppRenderOutput> {
        let ty = cpp_const_type(&self.ty);
        let value = render_const(&self.value);
        let mut out = CppRenderOutput::default();

        let is_extern = matches!(
            self.ty,
            hir::ConstType::StringType(_) | hir::ConstType::WideStringType(_)
        );
        let ctx = json!({
            "ty": ty,
            "ident": &self.ident,
            "value": value,
            "is_extern": is_extern,
        });
        let header = renderer.render_template("const.h.j2", &ctx)?;
        let source = renderer.render_template("const.cpp.j2", &ctx)?;
        out.header.push(header);
        if !source.trim().is_empty() {
            out.source.push(source);
        }

        Ok(out)
    }
}
