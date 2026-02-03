use crate::error::IdlcResult;
use crate::generate::rust_axum::definition::render_module_body;
use crate::generate::rust_axum::{RustAxumRender, RustAxumRenderOutput, RustAxumRenderer};
use itertools::Itertools;
use xidl_parser::hir;

impl RustAxumRender for hir::Specification {
    fn render(&self, renderer: &RustAxumRenderer) -> IdlcResult<RustAxumRenderOutput> {
        let defs = self.0.iter().collect_vec();
        let body = render_module_body(&defs, renderer)?;
        Ok(RustAxumRenderOutput { source: vec![body] })
    }
}
