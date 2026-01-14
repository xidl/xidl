use crate::error::IdlcResult;
use crate::generate::c::util::{comment_placeholder, interface_name};
use crate::generate::c::{CRender, CRenderer};
use xidl_parser::hir;

impl CRender for hir::Definition {
    fn render(&self, renderer: &CRenderer) -> IdlcResult<Vec<String>> {
        match self {
            hir::Definition::ConstrTypeDcl(constr) => constr.render(renderer),
            hir::Definition::ConstDcl(const_dcl) => const_dcl.render(renderer),
            hir::Definition::InterfaceDcl(interface) => Ok(vec![comment_placeholder(&format!(
                "interface {} skipped",
                interface_name(interface)
            ))]),
        }
    }
}
