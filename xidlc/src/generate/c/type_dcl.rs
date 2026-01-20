use crate::error::IdlcResult;
use crate::generate::c::{CRender, CRenderOutput, CRenderer};
use xidl_parser::hir;

impl CRender for hir::TypeDcl {
    fn render(&self, renderer: &CRenderer) -> IdlcResult<CRenderOutput> {
        match &self.decl {
            hir::TypeDclInner::ConstrTypeDcl(constr) => constr.render(renderer),
            _ => Ok(CRenderOutput::default()),
        }
    }
}
