use crate::error::IdlcResult;
use crate::generate::rust::bitset_dcl::render_bitset_with_config;
use crate::generate::rust::struct_dcl::render_struct_with_config;
use crate::generate::rust::union_def::render_union_with_config;
use crate::generate::rust::util::{
    constr_type_scoped_name, rust_scoped_name, rust_type, typedef_json,
};
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use crate::generate::utils::doc_lines_from_annotations;
use serde_json::json;
use xidl_parser::hir;

impl RustRender for hir::TypeDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        match &self.decl {
            hir::TypeDclInner::ConstrTypeDcl(constr) => constr.render(renderer),
            hir::TypeDclInner::TypedefDcl(typedef) => {
                render_typedef_with_config(typedef, renderer, &[])
            }
            hir::TypeDclInner::NativeDcl(native) => {
                let ctx = renderer.enrich_ctx(
                    json!({
                    "name": crate::generate::rust::util::rust_ident(&native.decl.0),
                    "ty": "*mut c_void",
                    "typeobject_complete": "xidl_typeobject::runtime::build_complete_alias(\"\", 0u32, xidl_typeobject::runtime::type_identifier_none())",
                    "typeobject_minimal": "xidl_typeobject::runtime::build_minimal_alias(0u32, xidl_typeobject::runtime::type_identifier_none())",
                    }),
                    &doc_lines_from_annotations(&native.annotations),
                );
                renderer.render_source_template("typedef.rs.j2", &ctx)
            }
        }
    }
}

impl RustRender for hir::TypedefDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        render_typedef_with_config(self, renderer, &[])
    }
}

pub(crate) fn render_typedef_with_config(
    def: &hir::TypedefDcl,
    renderer: &RustRenderer,
    module_path: &[String],
) -> IdlcResult<RustRenderOutput> {
    let mut out = RustRenderOutput::default();
    let doc = doc_lines_from_annotations(&def.annotations);
    match &def.ty {
        hir::TypedefType::TypeSpec(ty) => {
            for decl in &def.decl {
                let base = rust_type(ty);
                let ctx = renderer.enrich_ctx(typedef_json(&base, decl), &doc);
                out.extend(renderer.render_source_template("typedef.rs.j2", &ctx)?);
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
                hir::ConstrTypeDcl::EnumDcl(def) => def.render(renderer)?,
                hir::ConstrTypeDcl::BitsetDcl(def) => {
                    render_bitset_with_config(def, renderer, &config)?
                }
                hir::ConstrTypeDcl::BitmaskDcl(def) => def.render(renderer)?,
                hir::ConstrTypeDcl::StructForwardDcl(_)
                | hir::ConstrTypeDcl::UnionForwardDcl(_) => RustRenderOutput::default(),
            };
            out.extend(rendered);
            let base = rust_scoped_name(&constr_type_scoped_name(constr));
            for decl in &def.decl {
                let ctx = renderer.enrich_ctx(typedef_json(&base, decl), &doc);
                out.extend(renderer.render_source_template("typedef.rs.j2", &ctx)?);
            }
        }
    }
    Ok(out)
}
