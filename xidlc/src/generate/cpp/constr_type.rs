use crate::error::IdlcResult;
use crate::generate::cpp::{CppRender, CppRenderOutput, CppRenderer};
use xidl_parser::hir;

impl CppRender for hir::ConstrTypeDcl {
    fn render(&self, renderer: &CppRenderer) -> IdlcResult<CppRenderOutput> {
        match self {
            hir::ConstrTypeDcl::StructForwardDcl(def) => def.render(renderer),
            hir::ConstrTypeDcl::StructDcl(def) => def.render(renderer),
            hir::ConstrTypeDcl::EnumDcl(def) => def.render(renderer),
            hir::ConstrTypeDcl::UnionForwardDcl(def) => def.render(renderer),
            hir::ConstrTypeDcl::UnionDef(def) => def.render(renderer),
            hir::ConstrTypeDcl::BitsetDcl(def) => def.render(renderer),
            hir::ConstrTypeDcl::BitmaskDcl(def) => def.render(renderer),
        }
    }
}
