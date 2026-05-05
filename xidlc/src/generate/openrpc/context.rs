use serde_json::Value;
use std::collections::BTreeMap;
use xidl_parser::hir;
use xidl_parser::jsonrpc_hir::{JsonRpcHirDocument, JsonRpcInterface};

use super::annotations::doc_text;
use super::methods::render_method;
use super::names::{declarator_name, scoped_name};
use super::schema::{
    apply_schema_description, schema_for_constr_type, schema_for_decl, schema_for_struct,
    schema_for_union,
};

#[derive(Default)]
pub(super) struct OpenRpcContext {
    pub(super) schemas: BTreeMap<String, Value>,
    pub(super) methods: Vec<Value>,
    pub(super) info_title: Option<String>,
    pub(super) info_version: Option<String>,
}

impl OpenRpcContext {
    fn apply_pragma(&mut self, pragma: &hir::Pragma) {
        match pragma {
            hir::Pragma::XidlcPackage(value) if !value.is_empty() => {
                self.info_title = Some(value.clone());
            }
            hir::Pragma::XidlcOpenApiVersion(value) if !value.is_empty() => {
                self.info_version = Some(value.clone());
            }
            _ => {}
        }
    }

    pub(super) fn collect_doc(&mut self, doc: &JsonRpcHirDocument) {
        self.collect_spec(&doc.spec, &[]);
        for interface in &doc.interfaces {
            self.collect_interface(interface);
        }
    }

    fn collect_spec(&mut self, spec: &hir::Specification, module_path: &[String]) {
        for def in &spec.0 {
            self.collect_def(def, module_path);
        }
    }

    fn collect_def(&mut self, def: &hir::Definition, module_path: &[String]) {
        match def {
            hir::Definition::ModuleDcl(module) => self.collect_module(module, module_path),
            hir::Definition::TypeDcl(type_dcl) => self.collect_type_dcl(type_dcl, module_path),
            hir::Definition::ConstrTypeDcl(constr) => self.collect_constr_type(constr, module_path),
            hir::Definition::ExceptDcl(except) => {
                let name = scoped_name(module_path, &except.ident);
                self.schemas.insert(name, schema_for_struct(&except.member));
            }
            hir::Definition::Pragma(pragma) => self.apply_pragma(pragma),
            _ => {}
        }
    }

    fn collect_module(&mut self, module: &hir::ModuleDcl, module_path: &[String]) {
        let mut next_path = module_path.to_vec();
        next_path.push(module.ident.clone());
        for nested in &module.definition {
            self.collect_def(nested, &next_path);
        }
    }

    fn collect_type_dcl(&mut self, type_dcl: &hir::TypeDcl, module_path: &[String]) {
        match type_dcl {
            hir::TypeDcl::TypedefDcl(typedef) => {
                for decl in &typedef.decl {
                    let name = scoped_name(module_path, &declarator_name(decl));
                    let schema = match &typedef.ty {
                        hir::TypedefType::TypeSpec(ty) => schema_for_decl(ty, decl),
                        hir::TypedefType::ConstrTypeDcl(constr) => {
                            self.collect_constr_type(constr, module_path);
                            schema_for_constr_type(constr, module_path)
                        }
                    };
                    self.schemas.insert(name, schema);
                }
            }
            hir::TypeDcl::ConstrTypeDcl(constr) => {
                self.collect_constr_type(constr, module_path);
            }
            hir::TypeDcl::NativeDcl(_) => {}
        }
    }

    fn collect_constr_type(&mut self, constr: &hir::ConstrTypeDcl, module_path: &[String]) {
        let Some((name, schema)) = collected_schema(constr, module_path) else {
            return;
        };
        self.schemas.insert(name, schema);
    }

    fn collect_interface(&mut self, interface: &JsonRpcInterface) {
        for method in &interface.methods {
            if matches!(
                method.source,
                xidl_parser::jsonrpc_hir::JsonRpcMethodSource::AttributeStreamSource
            ) {
                continue;
            }
            self.methods.push(render_method(method));
        }
    }
}

fn collected_schema(
    constr: &hir::ConstrTypeDcl,
    module_path: &[String],
) -> Option<(String, Value)> {
    match constr {
        hir::ConstrTypeDcl::StructDcl(def) => Some((
            scoped_name(module_path, &def.ident),
            apply_schema_description(schema_for_struct(&def.member), doc_text(&def.annotations)),
        )),
        hir::ConstrTypeDcl::EnumDcl(def) => {
            let values = def
                .member
                .iter()
                .map(|value| Value::String(value.ident.clone()))
                .collect::<Vec<_>>();
            Some((
                scoped_name(module_path, &def.ident),
                apply_schema_description(
                    serde_json::json!({ "type": "string", "enum": values }),
                    doc_text(&def.annotations),
                ),
            ))
        }
        hir::ConstrTypeDcl::UnionDef(def) => Some((
            scoped_name(module_path, &def.ident),
            apply_schema_description(schema_for_union(def), doc_text(&def.annotations)),
        )),
        hir::ConstrTypeDcl::BitsetDcl(def) => Some((
            scoped_name(module_path, &def.ident),
            apply_schema_description(
                serde_json::json!({ "type": "integer" }),
                doc_text(&def.annotations),
            ),
        )),
        hir::ConstrTypeDcl::BitmaskDcl(def) => Some((
            scoped_name(module_path, &def.ident),
            apply_schema_description(
                serde_json::json!({ "type": "integer" }),
                doc_text(&def.annotations),
            ),
        )),
        hir::ConstrTypeDcl::StructForwardDcl(_) | hir::ConstrTypeDcl::UnionForwardDcl(_) => None,
    }
}
