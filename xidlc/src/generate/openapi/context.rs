use super::builder::{build_error_schema, build_operation};
use super::naming::{declarator_name, field_rename, openapi_method_name, scoped_name};
use super::operation::{MethodInfo, render_http_operation};
use super::schema::{
    apply_schema_description, doc_text, schema_for_constr_type, schema_for_struct, schema_for_union,
};
use super::security::register_security_schemes;
use super::stream::{OpenApiStreamPatch, request_stream_content_type, stream_patch_item_schema};
use crate::openapi::RefOr;
use crate::openapi::path::PathItem;
use crate::openapi::schema::{ObjectBuilder, Schema, Type};
use crate::openapi::server::Server;
use std::collections::{BTreeMap, HashMap};
use std::mem;
use xidl_parser::hir;
use xidl_parser::http_hir::HttpHirDocument;

#[derive(Default)]
pub(crate) struct OpenApiContext {
    pub(crate) schemas: BTreeMap<String, RefOr<Schema>>,
    pub(crate) security_schemes: BTreeMap<String, crate::openapi::security::SecurityScheme>,
    pub(crate) paths: crate::openapi::path::PathsBuilder,
    pub(crate) info_title: Option<String>,
    pub(crate) info_version: Option<String>,
    pub(crate) servers: Vec<Server>,
    pub(crate) stream_patches: Vec<OpenApiStreamPatch>,
}

impl OpenApiContext {
    pub(crate) fn new(http_hir: &HttpHirDocument) -> Self {
        Self {
            info_title: http_hir.document.package.clone(),
            info_version: http_hir.document.version.clone(),
            servers: http_hir
                .document
                .servers
                .iter()
                .map(|server| {
                    let mut item = Server::new(&server.base_url);
                    item.description = server.description.clone();
                    item
                })
                .collect(),
            ..Self::default()
        }
    }

    pub(crate) fn collect(
        mut self,
        spec: &hir::Specification,
        module_path: &[String],
        http_hir: &HttpHirDocument,
    ) -> Self {
        for def in &spec.0 {
            self.collect_def(def, module_path, http_hir);
        }
        self
    }

    fn collect_def(
        &mut self,
        def: &hir::Definition,
        module_path: &[String],
        http_hir: &HttpHirDocument,
    ) {
        match def {
            hir::Definition::ModuleDcl(module) => {
                let mut next_path = module_path.to_vec();
                next_path.push(module.ident.clone());
                for def in &module.definition {
                    self.collect_def(def, &next_path, http_hir);
                }
            }
            hir::Definition::TypeDcl(type_dcl) => self.collect_type_dcl(type_dcl, module_path),
            hir::Definition::ConstrTypeDcl(constr) => self.collect_constr_type(constr, module_path),
            hir::Definition::ExceptDcl(except) => self.collect_exception(except, module_path),
            hir::Definition::InterfaceDcl(interface) => {
                self.collect_interface(interface, module_path, http_hir)
            }
            hir::Definition::Pragma(_) => {}
            _ => {}
        }
    }

    fn collect_type_dcl(&mut self, type_dcl: &hir::TypeDcl, module_path: &[String]) {
        match type_dcl {
            hir::TypeDcl::TypedefDcl(typedef) => {
                for decl in &typedef.decl {
                    let name = scoped_name(module_path, &declarator_name(decl));
                    let schema = match &typedef.ty {
                        hir::TypedefType::TypeSpec(ty) => super::schema::schema_for_type(ty),
                        hir::TypedefType::ConstrTypeDcl(constr) => {
                            self.collect_constr_type(constr, module_path);
                            schema_for_constr_type(constr, module_path)
                        }
                    };
                    self.schemas.insert(name, schema);
                }
            }
            hir::TypeDcl::ConstrTypeDcl(constr) => self.collect_constr_type(constr, module_path),
            hir::TypeDcl::NativeDcl(_) => {}
        }
    }

    fn collect_constr_type(&mut self, constr: &hir::ConstrTypeDcl, module_path: &[String]) {
        match constr {
            hir::ConstrTypeDcl::StructDcl(def) => {
                let name = scoped_name(module_path, &def.ident);
                let schema = apply_schema_description(
                    schema_for_struct(&def.member),
                    doc_text(&def.annotations).as_deref(),
                );
                self.schemas.insert(name, schema);
            }
            hir::ConstrTypeDcl::EnumDcl(def) => {
                let name = scoped_name(module_path, &def.ident);
                let values = def
                    .member
                    .iter()
                    .map(|v| {
                        serde_json::Value::String(
                            field_rename(&v.annotations).unwrap_or_else(|| v.ident.clone()),
                        )
                    })
                    .collect::<Vec<_>>();
                let schema = RefOr::T(Schema::from(
                    ObjectBuilder::new()
                        .schema_type(Type::String)
                        .enum_values(Some(values)),
                ));
                let schema =
                    apply_schema_description(schema, doc_text(&def.annotations).as_deref());
                self.schemas.insert(name, schema);
            }
            hir::ConstrTypeDcl::UnionDef(def) => {
                let name = scoped_name(module_path, &def.ident);
                let schema = apply_schema_description(
                    schema_for_union(def),
                    doc_text(&def.annotations).as_deref(),
                );
                self.schemas.insert(name, schema);
            }
            hir::ConstrTypeDcl::BitsetDcl(def) => {
                let name = scoped_name(module_path, &def.ident);
                let schema = RefOr::T(Schema::from(
                    ObjectBuilder::new().schema_type(Type::Integer),
                ));
                let schema =
                    apply_schema_description(schema, doc_text(&def.annotations).as_deref());
                self.schemas.insert(name, schema);
            }
            hir::ConstrTypeDcl::BitmaskDcl(def) => {
                let name = scoped_name(module_path, &def.ident);
                let schema = RefOr::T(Schema::from(
                    ObjectBuilder::new().schema_type(Type::Integer),
                ));
                let schema =
                    apply_schema_description(schema, doc_text(&def.annotations).as_deref());
                self.schemas.insert(name, schema);
            }
            hir::ConstrTypeDcl::StructForwardDcl(_) | hir::ConstrTypeDcl::UnionForwardDcl(_) => {}
        }
    }

    fn collect_exception(&mut self, except: &hir::ExceptDcl, module_path: &[String]) {
        let name = scoped_name(module_path, &except.ident);
        self.schemas.insert(name, schema_for_struct(&except.member));
    }

    fn collect_interface(
        &mut self,
        interface: &hir::InterfaceDcl,
        module_path: &[String],
        http_hir: &HttpHirDocument,
    ) {
        let def = match &interface.decl {
            hir::InterfaceDclInner::InterfaceDef(def) => def,
            _ => return,
        };
        let Some(http_interface) = http_hir.find_interface(module_path, &def.header.ident) else {
            return;
        };
        let mut route_bindings = HashMap::new();
        for method in http_interface
            .operations
            .iter()
            .map(|op| render_http_operation(op, module_path, &def.header.ident))
        {
            self.collect_method(method, &mut route_bindings);
        }
        self.schemas
            .entry("Error".to_string())
            .or_insert_with(build_error_schema);
    }

    fn collect_method(&mut self, method: MethodInfo, route_bindings: &mut HashMap<String, String>) {
        if let Some(security_requirements) = &method.security_requirements {
            register_security_schemes(&mut self.security_schemes, security_requirements);
        }

        for path in method.paths.clone() {
            self.register_route_binding(&method, &path, route_bindings);
            self.collect_stream_patches(&method, &path);
            self.paths = mem::take(&mut self.paths).path(
                path,
                PathItem::new(method.http_method.clone(), build_operation(&method)),
            );
        }
    }

    fn register_route_binding(
        &self,
        method: &MethodInfo,
        path: &str,
        route_bindings: &mut HashMap<String, String>,
    ) {
        let key = format!("{} {path}", openapi_method_name(&method.http_method));
        if let Some(previous) = route_bindings.insert(key.clone(), method.operation_id.clone()) {
            panic!(
                "duplicate HTTP route binding: {key} (operations: {previous}, {})",
                method.operation_id
            );
        }
    }

    fn collect_stream_patches(&mut self, method: &MethodInfo, path: &str) {
        if let Some(item_schema) = &method.request_stream_item_schema {
            self.stream_patches.push(OpenApiStreamPatch {
                path: path.to_string(),
                method: openapi_method_name(&method.http_method),
                response_status: None,
                content_type: request_stream_content_type(&method.request_body),
                item_schema: stream_patch_item_schema(item_schema, "request"),
            });
        }
        if let Some(item_schema) = &method.response_stream_item_schema {
            self.stream_patches.push(OpenApiStreamPatch {
                path: path.to_string(),
                method: openapi_method_name(&method.http_method),
                response_status: Some(method.response_status),
                content_type: method.response_content_type.clone(),
                item_schema: stream_patch_item_schema(item_schema, "response"),
            });
        }
    }
}
