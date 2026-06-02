use crate::generate::typescript::definition::annotations::has_annotation;
use crate::generate::utils::doc_lines_from_annotations;
use xidl_parser::hir;

use super::contexts::XjsonMeta;
use super::method::TypeRefTarget;
use super::names::{declarator_name, ts_prop_name};
use super::type_expr::{ts_type_for_decl, zod_schema_for_decl};

pub(crate) fn struct_fields(
    members: &[hir::Member],
    container_annotations: &[hir::Annotation],
    module_path: &[String],
) -> Vec<FieldDecl> {
    let mut fields = Vec::new();
    for member in members {
        if hir::is_skipped(&member.annotations) {
            continue;
        }
        let doc = doc_lines_from_annotations(&member.annotations);
        for decl in &member.ident {
            let raw_name = declarator_name(decl);
            let wire_name =
                hir::effective_wire_name(raw_name, &member.annotations, container_annotations);
            let prop = ts_prop_name(raw_name);

            let mut has_meta = false;
            let mut meta = XjsonMeta {
                name: None,
                flatten: None,
                omitempty: None,
            };

            if prop != wire_name {
                meta.name = Some(wire_name);
                has_meta = true;
            }
            if has_annotation(&member.annotations, "flatten") {
                meta.flatten = Some(true);
                has_meta = true;
            }
            if has_annotation(&member.annotations, "omitempty") {
                meta.omitempty = Some(true);
                has_meta = true;
            }

            let xjson_meta = if has_meta { Some(meta) } else { None };

            fields.push(FieldDecl {
                prop,
                ty: ts_type_for_decl(&member.ty, decl, module_path, TypeRefTarget::Types),
                schema: zod_schema_for_decl(&member.ty, decl, module_path),
                optional: member.is_optional(),
                xjson_meta,
                doc: doc.clone(),
            });
        }
    }
    fields
}

pub(crate) struct FieldDecl {
    pub(crate) prop: String,
    pub(crate) ty: String,
    pub(crate) schema: String,
    pub(crate) optional: bool,
    pub(crate) xjson_meta: Option<XjsonMeta>,
    pub(crate) doc: Vec<String>,
}
