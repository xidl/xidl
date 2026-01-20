use crate::error::IdlcResult;
use crate::generate::c::util::{comment_placeholder, interface_name};
use crate::generate::c::{CRender, CRenderOutput, CRenderer};
use xidl_parser::hir;

impl CRender for hir::Definition {
    fn render(&self, renderer: &CRenderer) -> IdlcResult<CRenderOutput> {
        match self {
            hir::Definition::ConstrTypeDcl(constr) => constr.render(renderer),
            hir::Definition::TypeDcl(type_dcl) => type_dcl.render(renderer),
            hir::Definition::ConstDcl(const_dcl) => const_dcl.render(renderer),
            hir::Definition::ExceptDcl(except_dcl) => except_dcl.render(renderer),
            hir::Definition::InterfaceDcl(interface) => {
                let rendered = interface.render(renderer)?;
                if rendered.header.is_empty() && rendered.source.is_empty() {
                    Ok(
                        CRenderOutput::default().push_header(comment_placeholder(&format!(
                            "interface {} skipped",
                            interface_name(interface)
                        ))),
                    )
                } else {
                    Ok(rendered)
                }
            }
        }
    }
}
