use crate::error::IdlcResult;
use crate::generate::rust::util::{rust_scoped_name, rust_type, typedef_json};
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use serde_json::json;
use xidl_parser::hir;

impl RustRender for hir::TypeDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        match &self.decl {
            hir::TypeDclInner::ConstrTypeDcl(constr) => constr.render(renderer),
            hir::TypeDclInner::TypedefDcl(typedef) => typedef.render(renderer),
            hir::TypeDclInner::NativeDcl(native) => {
                let ctx = json!({
                    "name": crate::generate::rust::util::rust_ident(&native.decl.0),
                    "ty": "*mut c_void",
                });
                let rendered = renderer.render_template("typedef.rs.j2", &ctx)?;
                Ok(RustRenderOutput::default().push_source(rendered))
            }
        }
    }
}

impl RustRender for hir::TypedefDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        let mut out = RustRenderOutput::default();
        match &self.ty {
            hir::TypedefType::TypeSpec(ty) => {
                for decl in &self.decl {
                    let base = rust_type(ty);
                    let ctx = typedef_json(&base, decl);
                    let rendered = renderer.render_template("typedef.rs.j2", &ctx)?;
                    out.source.push(rendered);
                }
            }
            hir::TypedefType::ConstrTypeDcl(constr) => {
                out.extend(constr.render(renderer)?);
                let name = match constr {
                    hir::ConstrTypeDcl::StructForwardDcl(def) => def.ident.clone(),
                    hir::ConstrTypeDcl::StructDcl(def) => def.ident.clone(),
                    hir::ConstrTypeDcl::EnumDcl(def) => def.ident.clone(),
                    hir::ConstrTypeDcl::UnionForwardDcl(def) => def.ident.clone(),
                    hir::ConstrTypeDcl::UnionDef(def) => def.ident.clone(),
                    hir::ConstrTypeDcl::BitsetDcl(def) => def.ident.clone(),
                    hir::ConstrTypeDcl::BitmaskDcl(def) => def.ident.clone(),
                };
                let ty = hir::TypeSpec::SimpleTypeSpec(hir::SimpleTypeSpec::ScopedName(
                    hir::ScopedName {
                        name: vec![name],
                        is_root: false,
                    },
                ));
                let base = match &ty {
                    hir::TypeSpec::SimpleTypeSpec(hir::SimpleTypeSpec::ScopedName(value)) => {
                        rust_scoped_name(value)
                    }
                    _ => unreachable!(),
                };
                for decl in &self.decl {
                    let ctx = typedef_json(&base, decl);
                    let rendered = renderer.render_template("typedef.rs.j2", &ctx)?;
                    out.source.push(rendered);
                }
            }
        }
        Ok(out)
    }
}
