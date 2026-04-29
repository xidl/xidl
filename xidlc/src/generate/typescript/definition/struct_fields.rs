use crate::generate::utils::doc_lines_from_annotations;
use xidl_parser::hir;

use super::annotations::field_rename;
use super::method::TypeRefTarget;
use super::names::{declarator_name, ts_prop_name};
use super::type_expr::{ts_type_for_decl, zod_schema_for_decl};

pub(crate) fn struct_fields(members: &[hir::Member], module_path: &[String]) -> Vec<FieldDecl> {
    let mut fields = Vec::new();
    for member in members {
        let rename = field_rename(&member.annotations);
        let doc = doc_lines_from_annotations(&member.annotations);
        for decl in &member.ident {
            let name = rename
                .clone()
                .unwrap_or_else(|| declarator_name(decl).to_string());
            fields.push(FieldDecl {
                prop: ts_prop_name(&name),
                ty: ts_type_for_decl(&member.ty, decl, module_path, TypeRefTarget::Types),
                schema: zod_schema_for_decl(&member.ty, decl, module_path),
                optional: member.is_optional(),
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
    pub(crate) doc: Vec<String>,
}
