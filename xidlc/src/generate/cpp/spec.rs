use crate::error::IdlcResult;
use crate::generate::cpp::{CppRender, CppRenderOutput, CppRenderer};
use xidl_parser::hir;

impl CppRender for hir::Specification {
    fn render(&self, renderer: &CppRenderer) -> IdlcResult<CppRenderOutput> {
        let mut out = CppRenderOutput::default();
        for def in &self.0 {
            out.extend(def.render(renderer)?);
        }
        Ok(out)
    }
}
