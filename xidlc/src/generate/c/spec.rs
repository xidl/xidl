use crate::error::IdlcResult;
use crate::generate::c::{CRender, CRenderer};
use xidl_parser::hir;

impl CRender for hir::Specification {
    fn render(&self, renderer: &CRenderer) -> IdlcResult<Vec<String>> {
        let mut out = Vec::new();
        for def in &self.0 {
            out.extend(def.render(renderer)?);
        }
        Ok(out)
    }
}
