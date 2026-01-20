use crate::error::IdlcResult;
use crate::generate::rust::definition::render_module_body;
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use xidl_parser::hir;

impl RustRender for hir::Specification {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        let defs = self.0.iter().collect::<Vec<_>>();
        let body = render_module_body(&defs, renderer)?;
        Ok(RustRenderOutput { source: vec![body] })
    }
}
