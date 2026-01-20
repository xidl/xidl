use crate::error::IdlcResult;
use crate::generate::cpp::{CppRender, CppRenderOutput, CppRenderer};
use xidl_parser::hir;

impl CppRender for hir::ModuleDcl {
    fn render(&self, renderer: &CppRenderer) -> IdlcResult<CppRenderOutput> {
        let mut out = CppRenderOutput::default();
        for def in &self.definition {
            out.extend(def.render(renderer)?);
        }
        Ok(out)
    }
}

impl CppRender for hir::Definition {
    fn render(&self, renderer: &CppRenderer) -> IdlcResult<CppRenderOutput> {
        match self {
            hir::Definition::ModuleDcl(module) => module.render(renderer),
            hir::Definition::ConstrTypeDcl(constr) => constr.render(renderer),
            hir::Definition::TypeDcl(type_dcl) => type_dcl.render(renderer),
            hir::Definition::ConstDcl(const_dcl) => const_dcl.render(renderer),
            hir::Definition::ExceptDcl(except_dcl) => except_dcl.render(renderer),
            hir::Definition::InterfaceDcl(interface) => interface.render(renderer),
        }
    }
}
