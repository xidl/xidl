use crate::error::IdlcResult;
use crate::generate::c::util::comment_placeholder;
use crate::generate::c::{CRender, CRenderer};
use xidl_parser::hir;

impl CRender for hir::ConstrTypeDcl {
    fn render(&self, renderer: &CRenderer) -> IdlcResult<Vec<String>> {
        match self {
            hir::ConstrTypeDcl::StructForwardDcl(forward) => Ok(vec![comment_placeholder(
                &format!("struct {} forward skipped", forward.ident),
            )]),
            hir::ConstrTypeDcl::StructDcl(def) => def.render(renderer),
            hir::ConstrTypeDcl::EnumDcl(def) => def.render(renderer),
            hir::ConstrTypeDcl::UnionForwardDcl(def) => Ok(vec![comment_placeholder(&format!(
                "union {} skipped",
                def.ident
            ))]),
            hir::ConstrTypeDcl::UnionDef(def) => Ok(vec![comment_placeholder(&format!(
                "union {} skipped",
                def.ident
            ))]),
            hir::ConstrTypeDcl::BitsetDcl(def) => Ok(vec![comment_placeholder(&format!(
                "bitset {} skipped",
                def.ident
            ))]),
            hir::ConstrTypeDcl::BitmaskDcl(def) => Ok(vec![comment_placeholder(&format!(
                "bitmask {} skipped",
                def.ident
            ))]),
        }
    }
}
