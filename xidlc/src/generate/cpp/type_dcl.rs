use crate::error::IdlcResult;
use crate::generate::cpp::util::{collect_inline_defs, cpp_type, typedef_json};
use crate::generate::cpp::{CppRender, CppRenderOutput, CppRenderer};
use serde_json::json;
use xidl_parser::hir;

impl CppRender for hir::TypeDcl {
    fn render(&self, renderer: &CppRenderer) -> IdlcResult<CppRenderOutput> {
        match &self.decl {
            hir::TypeDclInner::ConstrTypeDcl(constr) => constr.render(renderer),
            hir::TypeDclInner::TypedefDcl(typedef) => typedef.render(renderer),
            hir::TypeDclInner::NativeDcl(native) => {
                let ctx = json!({
                    "name": &native.decl.0,
                    "ty": "void*",
                    "emit_traits": false,
                });
                let rendered = renderer.render_template("typedef.h.j2", &ctx)?;
                Ok(CppRenderOutput::default().push_header(rendered))
            }
        }
    }
}

impl CppRender for hir::TypedefDcl {
    fn render(&self, renderer: &CppRenderer) -> IdlcResult<CppRenderOutput> {
        let mut out = CppRenderOutput::default();
        match &self.ty {
            hir::TypedefType::TypeSpec(ty) => {
                for decl in &self.decl {
                    let base = cpp_type(ty);
                    let mut ctx = typedef_json(&base, decl);
                    ctx["emit_traits"] = json!(true);
                    let rendered = renderer.render_template("typedef.h.j2", &ctx)?;
                    out.header.push(rendered);
                }
            }
            hir::TypedefType::ConstrTypeDcl(constr) => {
                out.extend(collect_inline_defs(constr, renderer)?);
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
                for decl in &self.decl {
                    let base = match &ty {
                        hir::TypeSpec::SimpleTypeSpec(hir::SimpleTypeSpec::ScopedName(value)) => {
                            crate::generate::cpp::util::cpp_scoped_name(value)
                        }
                        _ => unreachable!(),
                    };
                    let mut ctx = typedef_json(&base, decl);
                    ctx["emit_traits"] = json!(true);
                    let rendered = renderer.render_template("typedef.h.j2", &ctx)?;
                    out.header.push(rendered);
                }
            }
        }
        Ok(out)
    }
}
