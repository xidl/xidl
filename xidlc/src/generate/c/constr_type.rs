use crate::error::IdlcResult;
use crate::generate::c::util::comment_placeholder;
use crate::generate::c::{CRender, CRenderOutput, CRenderer};
use xidl_parser::hir;

impl CRender for hir::ConstrTypeDcl {
    fn render(&self, renderer: &CRenderer) -> IdlcResult<CRenderOutput> {
        match self {
            hir::ConstrTypeDcl::StructForwardDcl(forward) => Ok(CRenderOutput::default()
                .push_header(comment_placeholder(&format!(
                    "struct {} forward skipped",
                    forward.ident
                )))),
            hir::ConstrTypeDcl::StructDcl(def) => def.render(renderer),
            hir::ConstrTypeDcl::EnumDcl(def) => def.render(renderer),
            hir::ConstrTypeDcl::UnionForwardDcl(def) => Ok(CRenderOutput::default()
                .push_header(comment_placeholder(&format!("union {} skipped", def.ident)))),
            hir::ConstrTypeDcl::UnionDef(def) => def.render(renderer),
            hir::ConstrTypeDcl::BitsetDcl(def) => def.render(renderer),
            hir::ConstrTypeDcl::BitmaskDcl(def) => def.render(renderer),
        }
    }
}
