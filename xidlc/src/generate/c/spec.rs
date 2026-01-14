use crate::error::IdlcResult;
use crate::generate::c::{CRender, CRenderOutput, CRenderer};
use xidl_parser::hir;

impl CRender for hir::Specification {
    fn render(&self, renderer: &CRenderer) -> IdlcResult<CRenderOutput> {
        let mut out = CRenderOutput::default();
        for def in &self.0 {
            out.extend(def.render(renderer)?);
        }
        Ok(out)
    }
}
