use crate::error::IdlcResult;
use crate::generate::c::{CRender, CRenderOutput, CRenderer};
use xidl_parser::hir;

impl CRender for hir::ExceptDcl {
    fn render(&self, _renderer: &CRenderer) -> IdlcResult<CRenderOutput> {
        Ok(CRenderOutput::default())
    }
}
