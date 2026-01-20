use crate::error::IdlcResult;
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use xidl_parser::hir;

impl RustRender for hir::ConstrTypeDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
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
