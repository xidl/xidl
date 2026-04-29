use crate::generate::http_hir::HttpHirDocument;
use crate::generate::http_hir::semantics::DeprecatedInfo;
use crate::generate::openapi::operation::{
    MethodInfo, register_security_schemes, render_http_operation,
};
use crate::generate::openapi::schema::{
    apply_schema_description, error_schema_ref, schema_for_constr_type, schema_for_struct,
    schema_for_type,
};
use crate::generate::openapi::utils::{doc_text, openapi_method_name, scoped_name};
use crate::openapi::path::{OperationBuilder, PathItem, PathsBuilder};
use crate::openapi::response::ResponseBuilder;
use crate::openapi::schema::{ObjectBuilder, Schema, Type};
use crate::openapi::security::SecurityScheme;
use crate::openapi::server::Server;
use crate::openapi::{Content, InfoBuilder, OpenApi, OpenApiBuilder, RefOr, ResponsesBuilder};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::mem;
use xidl_parser::hir;

/// The rendered OpenAPI document and any patches needed for stream support.
pub struct RenderedOpenApi {
    pub document: OpenApi,
    pub stream_patches: Vec<OpenApiStreamPatch>,
}

/// A patch to be applied to the OpenAPI document to support itemSchema for streams.
pub struct OpenApiStreamPatch {
    pub path: String,
    pub method: &'static str,
    pub response_status: Option<&'static str>,
    pub content_type: String,
    pub item_schema: Value,
}

#[derive(Default)]
pub struct OpenApiContext {
    pub schemas: BTreeMap<String, RefOr<Schema>>,
    pub security_schemes: BTreeMap<String, SecurityScheme>,
    pub paths: PathsBuilder,
    pub info_title: Option<String>,
    pub info_version: Option<String>,
    pub servers: Vec<Server>,
    pub stream_patches: Vec<OpenApiStreamPatch>,
}

impl OpenApiContext {
    /// Collects definitions from the HIR specification.
    pub fn collect_spec(
        &mut self,
        spec: &hir::Specification,
        module_path: &[String],
        http_hir: &HttpHirDocument,
    ) {
        for def in &spec.0 {
            self.collect_def(def, module_path, http_hir);
        }
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
            _ => {}
        }
    }

    fn collect_type_dcl(&mut self, type_dcl: &hir::TypeDcl, module_path: &[String]) {
        match type_dcl {
            hir::TypeDcl::TypedefDcl(typedef) => {
                for decl in &typedef.decl {
                    let name = scoped_name(
                        module_path,
                        &crate::generate::openapi::utils::declarator_name(decl),
                    );
                    let schema = match &typedef.ty {
                        hir::TypedefType::TypeSpec(ty) => schema_for_type(ty),
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
            _ => {}
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
                let values = def.member.iter().map(|v| {
                    let rename = crate::generate::openapi::utils::field_rename(&v.annotations);
                    let raw = rename.unwrap_or_else(|| v.ident.clone());
                    Value::String(raw)
                });
                let schema = RefOr::T(Schema::from(
                    ObjectBuilder::new()
                        .schema_type(Type::String)
                        .enum_values(Some(values.collect::<Vec<_>>())),
                ));
                let schema =
                    apply_schema_description(schema, doc_text(&def.annotations).as_deref());
                self.schemas.insert(name, schema);
            }
            hir::ConstrTypeDcl::UnionDef(def) => {
                let name = scoped_name(module_path, &def.ident);
                let schema = apply_schema_description(
                    crate::generate::openapi::schema::schema_for_union(def),
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
            _ => {}
        }
    }

    fn collect_exception(&mut self, except: &hir::ExceptDcl, module_path: &[String]) {
        let name = scoped_name(module_path, &except.ident);
        let schema = schema_for_struct(&except.member);
        self.schemas.insert(name, schema);
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
        let methods = http_interface
            .operations
            .iter()
            .map(|op| render_http_operation(op, module_path, &def.header.ident))
            .collect::<Vec<_>>();
        let mut route_bindings = HashMap::new();

        for method in methods {
            self.register_method(method, &mut route_bindings);
        }

        self.schemas.entry("Error".to_string()).or_insert_with(|| {
            RefOr::T(Schema::from(
                ObjectBuilder::new()
                    .schema_type(Type::Object)
                    .property("code", ObjectBuilder::new().schema_type(Type::Integer))
                    .required("code")
                    .property("msg", ObjectBuilder::new().schema_type(Type::String))
                    .required("msg")
                    .property("details", ObjectBuilder::new().schema_type(Type::Object)),
            ))
        });
    }

    fn register_method(
        &mut self,
        method: MethodInfo,
        route_bindings: &mut HashMap<String, String>,
    ) {
        let MethodInfo {
            http_method,
            paths,
            operation_id,
            parameters,
            request_body,
            request_stream_item_schema,
            response_status,
            response_schema,
            response_stream_item_schema,
            summary,
            description,
            deprecated,
            deprecated_info,
            security_requirements,
            security,
            response_content_type,
        } = method;

        if let Some(security_requirements) = &security_requirements {
            register_security_schemes(&mut self.security_schemes, security_requirements);
        }

        let description = apply_deprecation_note(description, deprecated_info.as_ref());
        for path in paths {
            let key = format!("{} {path}", openapi_method_name(&http_method));
            if let Some(previous) = route_bindings.insert(key.clone(), operation_id.clone()) {
                panic!(
                    "duplicate HTTP route binding: {key} (operations: {previous}, {operation_id})"
                );
            }
            if let Some(item_schema) = &request_stream_item_schema {
                self.stream_patches.push(OpenApiStreamPatch {
                    path: path.clone(),
                    method: openapi_method_name(&http_method),
                    response_status: None,
                    content_type: request_stream_content_type(&request_body),
                    item_schema: serde_json::to_value(item_schema).unwrap_or_else(|err| {
                        panic!("failed to serialize request stream schema: {err}")
                    }),
                });
            }
            if let Some(item_schema) = &response_stream_item_schema {
                self.stream_patches.push(OpenApiStreamPatch {
                    path: path.clone(),
                    method: openapi_method_name(&http_method),
                    response_status: Some(response_status),
                    content_type: response_content_type.clone(),
                    item_schema: serde_json::to_value(item_schema).unwrap_or_else(|err| {
                        panic!("failed to serialize response stream schema: {err}")
                    }),
                });
            }
            let mut responses = ResponsesBuilder::new();
            let mut ok_response = ResponseBuilder::new().description("OK");
            if let Some(schema) = &response_schema {
                ok_response =
                    ok_response.content(&response_content_type, Content::new(Some(schema.clone())));
            }
            responses = responses.response(response_status, ok_response.build());
            let mut operation = OperationBuilder::new()
                .operation_id(Some(operation_id.clone()))
                .deprecated(if deprecated {
                    Some(crate::openapi::Deprecated::True)
                } else {
                    None
                })
                .responses(
                    responses
                        .response(
                            "500",
                            ResponseBuilder::new()
                                .description("Error")
                                .content("application/json", Content::new(Some(error_schema_ref())))
                                .build(),
                        )
                        .build(),
                );
            if summary.is_some() || description.is_some() {
                operation = operation
                    .summary(summary.as_deref())
                    .description(description.as_deref());
            }
            if let Some(security) = &security {
                operation = operation.securities(Some(security.clone()));
            }
            for parameter in &parameters {
                operation = operation.parameter(parameter.clone());
            }
            if let Some(request_body) = &request_body {
                operation = operation.request_body(Some(request_body.clone()));
            }
            let paths_builder = mem::take(&mut self.paths);
            self.paths = paths_builder.path(path, PathItem::new(http_method.clone(), operation));
        }
    }
}

pub fn render_openapi(spec: &hir::Specification, http_hir: &HttpHirDocument) -> RenderedOpenApi {
    let mut ctx = OpenApiContext {
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
        ..Default::default()
    };
    ctx.collect_spec(spec, &[], http_hir);

    let mut components = crate::openapi::ComponentsBuilder::new();
    for (name, schema) in ctx.schemas {
        components = components.schema(name, schema);
    }
    for (name, scheme) in ctx.security_schemes {
        components = components.security_scheme(name, scheme);
    }

    let title = ctx.info_title.as_deref().unwrap_or("xidl");
    let version = ctx.info_version.as_deref().unwrap_or("0.1.0");

    let servers = if ctx.servers.is_empty() {
        None
    } else {
        Some(ctx.servers)
    };

    let document = OpenApiBuilder::new()
        .info(InfoBuilder::new().title(title).version(version).build())
        .paths(ctx.paths.build())
        .components(Some(components.build()))
        .servers(servers)
        .build();

    RenderedOpenApi {
        document,
        stream_patches: ctx.stream_patches,
    }
}

fn apply_deprecation_note(
    description: Option<String>,
    deprecated: Option<&DeprecatedInfo>,
) -> Option<String> {
    if description.is_some() {
        return description;
    }
    let Some(info) = deprecated else {
        return description;
    };
    let mut note = String::from("Deprecated.");
    match (&info.since, &info.after) {
        (Some(since), Some(after)) => {
            note.push_str(&format!(" Since {since}. After {after}."));
        }
        (Some(since), None) => {
            note.push_str(&format!(" Since {since}."));
        }
        (None, Some(after)) => {
            note.push_str(&format!(" After {after}."));
        }
        (None, None) => {}
    }
    Some(note)
}

fn request_stream_content_type(
    request_body: &Option<crate::openapi::request_body::RequestBody>,
) -> String {
    request_body
        .as_ref()
        .and_then(|body| body.content.keys().next().cloned())
        .unwrap_or_else(|| panic!("stream request body is missing content"))
}

pub fn patch_openapi_stream_content(doc: &mut Value, patch: &OpenApiStreamPatch) {
    let Some(paths) = doc.get_mut("paths").and_then(Value::as_object_mut) else {
        return;
    };
    let Some(path_item) = paths.get_mut(&patch.path).and_then(Value::as_object_mut) else {
        return;
    };
    let Some(operation) = path_item
        .get_mut(patch.method)
        .and_then(Value::as_object_mut)
    else {
        return;
    };

    let target = if let Some(status) = patch.response_status {
        operation
            .get_mut("responses")
            .and_then(Value::as_object_mut)
            .and_then(|responses| responses.get_mut(status))
            .and_then(Value::as_object_mut)
    } else {
        operation
            .get_mut("requestBody")
            .and_then(Value::as_object_mut)
    };
    let Some(target) = target else {
        return;
    };
    let Some(content) = target.get_mut("content").and_then(Value::as_object_mut) else {
        return;
    };
    let Some(media_type) = content
        .get_mut(&patch.content_type)
        .and_then(Value::as_object_mut)
    else {
        return;
    };
    media_type.insert("itemSchema".to_string(), patch.item_schema.clone());
    media_type.remove("schema");
}
