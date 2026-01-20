use crate::error::IdlcResult;
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use xidl_parser::hir;

impl RustRender for hir::Definition {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        match self {
            hir::Definition::ConstrTypeDcl(constr) => constr.render(renderer),
            hir::Definition::TypeDcl(type_dcl) => type_dcl.render(renderer),
            hir::Definition::ConstDcl(const_dcl) => const_dcl.render(renderer),
            hir::Definition::ExceptDcl(except_dcl) => except_dcl.render(renderer),
            hir::Definition::InterfaceDcl(interface) => interface.render(renderer),
        }
    }
}
