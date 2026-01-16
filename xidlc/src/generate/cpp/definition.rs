use crate::error::IdlcResult;
use crate::generate::cpp::{CppRender, CppRenderOutput, CppRenderer};
use xidl_parser::hir;

impl CppRender for hir::Definition {
    fn render(&self, renderer: &CppRenderer) -> IdlcResult<CppRenderOutput> {
        match self {
            hir::Definition::ConstrTypeDcl(constr) => constr.render(renderer),
            hir::Definition::ConstDcl(const_dcl) => const_dcl.render(renderer),
            hir::Definition::InterfaceDcl(interface) => interface.render(renderer),
        }
    }
}
