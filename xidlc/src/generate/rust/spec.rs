use crate::error::IdlcResult;
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use xidl_parser::hir;

impl RustRender for hir::Specification {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        let mut out = RustRenderOutput::default();
        for def in &self.0 {
            out.extend(def.render(renderer)?);
        }
        Ok(out)
    }
}
