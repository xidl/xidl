use crate::error::IdlcResult;
use crate::generate::rust::bitmask_dcl::render_bitmask_with_config;
use crate::generate::rust::bitset_dcl::render_bitset_with_config;
use crate::generate::rust::enum_dcl::render_enum_with_config;
use crate::generate::rust::struct_dcl::render_struct_with_config;
use crate::generate::rust::union_def::render_union_with_config;
use crate::generate::rust::util::{rust_scoped_name, rust_type, typedef_json};
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use serde_json::json;
use xidl_parser::hir;

impl RustRender for hir::TypeDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        match &self.decl {
            hir::TypeDclInner::ConstrTypeDcl(constr) => constr.render(renderer),
            hir::TypeDclInner::TypedefDcl(typedef) => {
                render_typedef_with_config(typedef, renderer, &[], &self.annotations)
            }
            hir::TypeDclInner::NativeDcl(native) => {
                let ctx = json!({
                    "name": crate::generate::rust::util::rust_ident(&native.decl.0),
                    "ty": "*mut c_void",
                    "typeobject_path": renderer.typeobject_path(),
                    "typeobject_complete": "xidl_typeobject::runtime::build_complete_alias(\"\", 0u32, xidl_typeobject::runtime::type_identifier_none())",
                    "typeobject_minimal": "xidl_typeobject::runtime::build_minimal_alias(0u32, xidl_typeobject::runtime::type_identifier_none())",
                });
                let rendered = renderer.render_template("typedef.rs.j2", &ctx)?;
                Ok(RustRenderOutput::default().push_source(rendered))
            }
        }
    }
}

impl RustRender for hir::TypedefDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        render_typedef_with_config(self, renderer, &[], &[])
    }
}

pub(crate) fn render_typedef_with_config(
    def: &hir::TypedefDcl,
    renderer: &RustRenderer,
    module_path: &[String],
    _annotations: &[hir::Annotation],
) -> IdlcResult<RustRenderOutput> {
    let mut out = RustRenderOutput::default();
    match &def.ty {
        hir::TypedefType::TypeSpec(ty) => {
            for decl in &def.decl {
                let base = rust_type(ty);
                let mut ctx = typedef_json(&base, decl);
                if let Some(obj) = ctx.as_object_mut() {
                    obj.insert(
                        "typeobject_path".to_string(),
                        json!(renderer.typeobject_path()),
                    );
                }
                let rendered = renderer.render_template("typedef.rs.j2", &ctx)?;
                out.source.push(rendered);
            }
        }
        hir::TypedefType::ConstrTypeDcl(constr) => {
            let config = hir::SerializeConfig::default();
            let rendered = match constr {
                hir::ConstrTypeDcl::StructDcl(def) => {
                    render_struct_with_config(def, renderer, &config, module_path)?
                }
                hir::ConstrTypeDcl::UnionDef(def) => {
                    render_union_with_config(def, renderer, &config, module_path)?
                }
                hir::ConstrTypeDcl::EnumDcl(def) => {
                    render_enum_with_config(def, renderer, module_path)?
                }
                hir::ConstrTypeDcl::BitsetDcl(def) => {
                    render_bitset_with_config(def, renderer, &config, module_path)?
                }
                hir::ConstrTypeDcl::BitmaskDcl(def) => {
                    render_bitmask_with_config(def, renderer, module_path)?
                }
                hir::ConstrTypeDcl::StructForwardDcl(_)
                | hir::ConstrTypeDcl::UnionForwardDcl(_) => RustRenderOutput::default(),
            };
            out.extend(rendered);
            let name = match constr {
                hir::ConstrTypeDcl::StructForwardDcl(def) => def.ident.clone(),
                hir::ConstrTypeDcl::StructDcl(def) => def.ident.clone(),
                hir::ConstrTypeDcl::EnumDcl(def) => def.ident.clone(),
                hir::ConstrTypeDcl::UnionForwardDcl(def) => def.ident.clone(),
                hir::ConstrTypeDcl::UnionDef(def) => def.ident.clone(),
                hir::ConstrTypeDcl::BitsetDcl(def) => def.ident.clone(),
                hir::ConstrTypeDcl::BitmaskDcl(def) => def.ident.clone(),
            };
            let ty =
                hir::TypeSpec::SimpleTypeSpec(hir::SimpleTypeSpec::ScopedName(hir::ScopedName {
                    name: vec![name],
                    is_root: false,
                }));
            let base = match &ty {
                hir::TypeSpec::SimpleTypeSpec(hir::SimpleTypeSpec::ScopedName(value)) => {
                    rust_scoped_name(value)
                }
                _ => unreachable!(),
            };
            for decl in &def.decl {
                let mut ctx = typedef_json(&base, decl);
                if let Some(obj) = ctx.as_object_mut() {
                    obj.insert(
                        "typeobject_path".to_string(),
                        json!(renderer.typeobject_path()),
                    );
                }
                let rendered = renderer.render_template("typedef.rs.j2", &ctx)?;
                out.source.push(rendered);
            }
        }
    }
    Ok(out)
}
