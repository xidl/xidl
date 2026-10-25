use super::builder::{build_error_schema, build_operation};
use super::naming::{declarator_name, openapi_method_name, scoped_name};
use super::operation::{MethodInfo, render_http_operation};
use super::schema::{apply_schema_description, doc_text, parameter_schema};
use super::scope::SchemaScope;
use super::security::register_security_schemes;
use super::stream::{OpenApiStreamPatch, request_stream_content_type, stream_patch_item_schema};
use crate::error::IdlcResult;
use crate::openapi::RefOr;
use crate::openapi::path::{ParameterIn, PathItem};
use crate::openapi::schema::{ObjectBuilder, Schema, Type};
use crate::openapi::server::Server;
use std::collections::{BTreeMap, BTreeSet};
use std::mem;
use xidl_parser::hir;
use xidl_parser::rest_hir::{RestHirDocument, semantics::has_annotation};

#[derive(Default)]
pub(crate) struct OpenApiContext {
    pub(crate) schemas: BTreeMap<String, RefOr<Schema>>,
    pub(crate) security_schemes: BTreeMap<String, crate::openapi::security::SecurityScheme>,
    pub(crate) paths: crate::openapi::path::PathsBuilder,
    pub(crate) info_title: Option<String>,
    pub(crate) info_version: Option<String>,
    pub(crate) servers: Vec<Server>,
    pub(crate) tags: BTreeSet<String>,
    pub(crate) stream_patches: Vec<OpenApiStreamPatch>,
}

impl OpenApiContext {
    pub(crate) fn new(rest_hir: &RestHirDocument) -> Self {
        Self {
            info_title: rest_hir.document.package.clone(),
            info_version: rest_hir.document.version.clone(),
            servers: rest_hir
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
        scope: SchemaScope<'_>,
        rest_hir: &RestHirDocument,
    ) -> IdlcResult<Self> {
        for def in &spec.0 {
            self.collect_def(def, scope, rest_hir)?;
        }
        Ok(self)
    }

    fn collect_def(
        &mut self,
        def: &hir::Definition,
        scope: SchemaScope<'_>,
        rest_hir: &RestHirDocument,
    ) -> IdlcResult<()> {
        match def {
            hir::Definition::ModuleDcl(module) => {
                let mut next_path = scope.module_path.to_vec();
                next_path.push(module.ident.clone());
                for def in &module.definition {
                    self.collect_def(def, scope.in_module(&next_path), rest_hir)?;
                }
            }
            hir::Definition::TypeDcl(type_dcl) => self.collect_type_dcl(type_dcl, scope)?,
            hir::Definition::ConstrTypeDcl(constr) => self.collect_constr_type(constr, scope)?,
            hir::Definition::ExceptDcl(except) => self.collect_exception(except, scope)?,
            hir::Definition::InterfaceDcl(interface) => {
                self.collect_interface(interface, scope, rest_hir)?
            }
            hir::Definition::Pragma(_) => {}
            _ => {}
        }
        Ok(())
    }

    fn collect_type_dcl(
        &mut self,
        type_dcl: &hir::TypeDcl,
        scope: SchemaScope<'_>,
    ) -> IdlcResult<()> {
        match type_dcl {
            hir::TypeDcl::TypedefDcl(typedef) => {
                for decl in &typedef.decl {
                    let name = scoped_name(scope.module_path, &declarator_name(decl));
                    let schema = match &typedef.ty {
                        hir::TypedefType::TypeSpec(ty) => scope.schema_for_type(ty)?,
                        hir::TypedefType::ConstrTypeDcl(constr) => {
                            self.collect_constr_type(constr, scope)?;
                            scope.schema_for_constr_type(constr)
                        }
                    };
                    self.schemas
                        .insert(name, SchemaScope::apply_declarator(schema, decl));
                }
            }
            hir::TypeDcl::ConstrTypeDcl(constr) => self.collect_constr_type(constr, scope)?,
            hir::TypeDcl::NativeDcl(_) => {}
        }
        Ok(())
    }

    fn collect_constr_type(
        &mut self,
        constr: &hir::ConstrTypeDcl,
        scope: SchemaScope<'_>,
    ) -> IdlcResult<()> {
        match constr {
            hir::ConstrTypeDcl::StructDcl(def) => {
                let name = scoped_name(scope.module_path, &def.ident);
                let own =
                    scope.schema_for_struct_with_annotations(&def.member, &def.annotations)?;
                let schema = scope.with_base_schemas(own, &def.parent)?;
                let schema =
                    apply_schema_description(schema, doc_text(&def.annotations).as_deref());
                self.schemas.insert(name, schema);
            }
            hir::ConstrTypeDcl::EnumDcl(def) => {
                let name = scoped_name(scope.module_path, &def.ident);
                let values = def
                    .member
                    .iter()
                    .map(|v| {
                        serde_json::Value::String(hir::effective_wire_name(
                            &v.ident,
                            &v.annotations,
                            &def.annotations,
                        ))
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
                let name = scoped_name(scope.module_path, &def.ident);
                let schema = apply_schema_description(
                    scope.schema_for_union(def)?,
                    doc_text(&def.annotations).as_deref(),
                );
                self.schemas.insert(name, schema);
            }
            hir::ConstrTypeDcl::BitsetDcl(def) => {
                let name = scoped_name(scope.module_path, &def.ident);
                let schema = RefOr::T(Schema::from(
                    ObjectBuilder::new().schema_type(Type::Integer),
                ));
                let schema =
                    apply_schema_description(schema, doc_text(&def.annotations).as_deref());
                self.schemas.insert(name, schema);
            }
            hir::ConstrTypeDcl::BitmaskDcl(def) => {
                let name = scoped_name(scope.module_path, &def.ident);
                let schema = RefOr::T(Schema::from(
                    ObjectBuilder::new().schema_type(Type::Integer),
                ));
                let schema =
                    apply_schema_description(schema, doc_text(&def.annotations).as_deref());
                self.schemas.insert(name, schema);
            }
            hir::ConstrTypeDcl::StructForwardDcl(_) | hir::ConstrTypeDcl::UnionForwardDcl(_) => {}
        }
        Ok(())
    }

    fn collect_exception(
        &mut self,
        except: &hir::ExceptDcl,
        scope: SchemaScope<'_>,
    ) -> IdlcResult<()> {
        let name = scoped_name(scope.module_path, &except.ident);
        // `@header`/`@cookie` members travel as response metadata, not body.
        let body_members: Vec<hir::Member> = except
            .member
            .iter()
            .filter(|member| {
                !has_annotation(&member.annotations, "header")
                    && !has_annotation(&member.annotations, "cookie")
            })
            .cloned()
            .collect();
        self.schemas.insert(
            name,
            scope.schema_for_struct_with_annotations(&body_members, &except.annotations)?,
        );
        Ok(())
    }

    fn collect_interface(
        &mut self,
        interface: &hir::InterfaceDcl,
        scope: SchemaScope<'_>,
        rest_hir: &RestHirDocument,
    ) -> IdlcResult<()> {
        let def = match &interface.decl {
            hir::InterfaceDclInner::InterfaceDef(def) => def,
            _ => return Ok(()),
        };
        let Some(http_interface) = rest_hir.find_interface(scope.module_path, &def.header.ident)
        else {
            return Ok(());
        };
        self.tags.insert(def.header.ident.clone());
        for method in http_interface.operations.iter().map(|op| {
            render_http_operation(op, scope, &def.header.ident, &rest_hir.document.exceptions)
        }) {
            let mut method = method?;
            self.apply_version_parameter(&mut method, rest_hir);
            self.collect_method(method)?;
        }
        self.schemas
            .entry("Error".to_string())
            .or_insert_with(build_error_schema);
        Ok(())
    }

    /// Declares the optional request parameter and fixed header response value.
    fn apply_version_parameter(&self, method: &mut MethodInfo, rest_hir: &RestHirDocument) {
        let Some(parameter) = &rest_hir.document.version_parameter else {
            return;
        };
        let location = match parameter.location {
            xidl_parser::hir::HttpParameterLocation::Header => ParameterIn::Header,
            xidl_parser::hir::HttpParameterLocation::Query => ParameterIn::Query,
            xidl_parser::hir::HttpParameterLocation::Cookie => ParameterIn::Cookie,
        };
        let description = match &rest_hir.document.version {
            Some(version) => format!("API version; this document describes {version}."),
            None => "API version.".to_string(),
        };
        let mut schema = ObjectBuilder::new().schema_type(Type::String);
        if let Some(version) = &rest_hir.document.version {
            schema = schema.enum_values(Some([version.as_str()]));
        }
        let schema_ref = RefOr::T(Schema::from(schema));
        let insert_at = method
            .parameters
            .iter()
            .take_while(|parameter| {
                matches!(
                    parameter.parameter_in,
                    ParameterIn::Path | ParameterIn::Query
                )
            })
            .count();
        method.parameters.insert(
            insert_at,
            parameter_schema(
                location,
                &parameter.name,
                schema_ref.clone(),
                false,
                Some(description.clone()),
            ),
        );
        if parameter.location == xidl_parser::hir::HttpParameterLocation::Header {
            method.version_response_header = Some((parameter.name.clone(), schema_ref));
        }
    }

    fn collect_method(&mut self, method: MethodInfo) -> IdlcResult<()> {
        if let Some(security_requirements) = &method.security_requirements {
            register_security_schemes(&mut self.security_schemes, security_requirements);
        }

        for path in method.paths.clone() {
            self.collect_stream_patches(&method, &path);
            self.paths = mem::take(&mut self.paths).path(
                path,
                PathItem::new(method.http_method.clone(), build_operation(&method)),
            );
        }
        Ok(())
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
                response_status: Some(method.response_status.clone()),
                content_type: method.response_content_type.clone(),
                item_schema: stream_patch_item_schema(item_schema, "response"),
            });
        }
    }
}
