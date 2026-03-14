use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::mem;
use utoipa::openapi::path::{
    HttpMethod as OpenApiHttpMethod, OperationBuilder, ParameterBuilder, ParameterIn, PathItem,
    PathsBuilder,
};
use utoipa::openapi::request_body::RequestBody;
use utoipa::openapi::response::ResponseBuilder;
use utoipa::openapi::schema::{
    ArrayBuilder, KnownFormat, ObjectBuilder, OneOf, Schema, SchemaFormat, Type,
};
use utoipa::openapi::security::{
    ApiKey, ApiKeyValue, Http, HttpAuthScheme, OAuth2, Scopes, SecurityRequirement, SecurityScheme,
};
use utoipa::openapi::server::Server;
use utoipa::openapi::{
    Content, InfoBuilder, OpenApi, OpenApiBuilder, Ref, RefOr, Required, ResponsesBuilder,
};
use xidl_parser::hir;
use xidl_parser::hir::{ParserProperties, Specification};

use crate::generate::utils::{
    DeprecatedInfo, HttpApiKeyLocation, HttpSecurityRequirement, HttpStreamCodec, HttpStreamKind,
    HttpStreamTargetSupport, deprecated_info, doc_lines_from_annotations, effective_media_type,
    effective_security, http_stream_config, validate_http_annotations, validate_http_stream_method,
    validate_http_stream_target,
};
use crate::jsonrpc::{Artifact, ArtifactFile};

pub(crate) struct OpenApiCodegen;

#[async_trait::async_trait]
impl crate::jsonrpc::Codegen for OpenApiCodegen {
    async fn get_engine_version(&self) -> Result<String, xidl_jsonrpc::Error> {
        Ok("*".to_string())
    }

    async fn get_properties(&self) -> Result<ParserProperties, xidl_jsonrpc::Error> {
        Ok(HashMap::new())
    }

    async fn generate(
        &self,
        hir: Specification,
        _path: String,
        _props: ParserProperties,
    ) -> Result<Vec<Artifact>, xidl_jsonrpc::Error> {
        let openapi = render_openapi_json(&hir)?;
        let content = serde_json::to_string_pretty(&openapi)?;
        Ok(vec![Artifact::new_file(ArtifactFile {
            path: "openapi.json".to_string(),
            content,
        })])
    }
}

fn render_openapi_json(spec: &hir::Specification) -> Result<Value, serde_json::Error> {
    let ctx = render_openapi(spec);
    let mut value = serde_json::to_value(ctx.document)?;
    if let Some(openapi) = value.get_mut("openapi") {
        *openapi = Value::String("3.2.0".to_string());
    }
    for patch in ctx.stream_patches {
        patch_openapi_stream_content(&mut value, &patch);
    }
    Ok(value)
}

pub fn render_openapi(spec: &hir::Specification) -> RenderedOpenApi {
    let mut ctx = OpenApiContext::default();
    ctx.collect_spec(spec, &[]);

    let mut components = utoipa::openapi::ComponentsBuilder::new();
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

#[derive(Default)]
struct OpenApiContext {
    schemas: BTreeMap<String, RefOr<Schema>>,
    security_schemes: BTreeMap<String, SecurityScheme>,
    paths: PathsBuilder,
    info_title: Option<String>,
    info_version: Option<String>,
    servers: Vec<Server>,
    stream_patches: Vec<OpenApiStreamPatch>,
}

pub struct RenderedOpenApi {
    pub document: OpenApi,
    stream_patches: Vec<OpenApiStreamPatch>,
}

struct OpenApiStreamPatch {
    path: String,
    method: &'static str,
    response_status: Option<&'static str>,
    content_type: String,
    item_schema: Value,
}

impl OpenApiContext {
    fn apply_pragma(&mut self, pragma: &hir::Pragma) {
        match pragma {
            hir::Pragma::XidlcPackage(value) => {
                if !value.is_empty() {
                    self.info_title = Some(value.clone());
                }
            }
            hir::Pragma::XidlcOpenApiVersion(value) => {
                if !value.is_empty() {
                    self.info_version = Some(value.clone());
                }
            }
            hir::Pragma::XidlcOpenApiService {
                base_url,
                description,
            } => {
                if !base_url.is_empty() {
                    let mut server = Server::new(base_url);
                    server.description = description.clone();
                    self.servers.push(server);
                }
            }
            _ => {}
        }
    }

    fn collect_spec(&mut self, spec: &hir::Specification, module_path: &[String]) {
        for def in &spec.0 {
            self.collect_def(def, module_path);
        }
    }

    fn collect_def(&mut self, def: &hir::Definition, module_path: &[String]) {
        match def {
            hir::Definition::ModuleDcl(module) => {
                let mut next_path = module_path.to_vec();
                next_path.push(module.ident.clone());
                for def in &module.definition {
                    self.collect_def(def, &next_path);
                }
            }
            hir::Definition::TypeDcl(type_dcl) => self.collect_type_dcl(type_dcl, module_path),
            hir::Definition::ConstrTypeDcl(constr) => self.collect_constr_type(constr, module_path),
            hir::Definition::ExceptDcl(except) => self.collect_exception(except, module_path),
            hir::Definition::InterfaceDcl(interface) => {
                self.collect_interface(interface, module_path)
            }
            hir::Definition::Pragma(pragma) => {
                self.apply_pragma(pragma);
            }
            _ => {}
        }
    }

    fn collect_type_dcl(&mut self, type_dcl: &hir::TypeDcl, module_path: &[String]) {
        match &type_dcl.decl {
            hir::TypeDclInner::TypedefDcl(typedef) => {
                for decl in &typedef.decl {
                    let name = scoped_name(module_path, &declarator_name(decl));
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
            hir::TypeDclInner::ConstrTypeDcl(constr) => {
                self.collect_constr_type(constr, module_path);
            }
            hir::TypeDclInner::NativeDcl(_) => {}
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
                    let rename = field_rename(&v.annotations);
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
        let schema = schema_for_struct(&except.member);
        self.schemas.insert(name, schema);
    }

    fn collect_interface(&mut self, interface: &hir::InterfaceDcl, module_path: &[String]) {
        let def = match &interface.decl {
            hir::InterfaceDclInner::InterfaceDef(def) => def,
            _ => return,
        };
        validate_http_annotations(
            &format!("interface '{}'", def.header.ident),
            &interface.annotations,
        )
        .unwrap_or_else(|err| panic!("{err}"));
        let mut methods = Vec::new();
        if let Some(body) = &def.interface_body {
            for export in &body.0 {
                match export {
                    hir::Export::OpDcl(op) => {
                        methods.push(render_op(
                            op,
                            &interface.annotations,
                            &def.header.ident,
                            module_path,
                        ));
                    }
                    hir::Export::AttrDcl(attr) => {
                        methods.extend(render_attr(
                            attr,
                            &interface.annotations,
                            &def.header.ident,
                            module_path,
                        ));
                    }
                    _ => {}
                }
            }
        }
        let mut route_bindings = HashMap::new();

        for method in methods {
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
                security_requirements,
                security,
                response_content_type,
            } = method;
            if let Some(security_requirements) = &security_requirements {
                register_security_schemes(&mut self.security_schemes, security_requirements);
            }

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
                    ok_response = ok_response
                        .content(&response_content_type, Content::new(Some(schema.clone())));
                }
                responses = responses.response(response_status, ok_response.build());
                let mut operation = OperationBuilder::new()
                    .operation_id(Some(operation_id.clone()))
                    .deprecated(if deprecated {
                        Some(utoipa::openapi::Deprecated::True)
                    } else {
                        None
                    })
                    .responses(
                        responses
                            .response(
                                "500",
                                ResponseBuilder::new()
                                    .description("Error")
                                    .content(
                                        "application/json",
                                        Content::new(Some(error_schema_ref())),
                                    )
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
                self.paths =
                    paths_builder.path(path, PathItem::new(http_method.clone(), operation));
            }
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
}

struct MethodInfo {
    http_method: OpenApiHttpMethod,
    paths: Vec<String>,
    operation_id: String,
    parameters: Vec<utoipa::openapi::path::Parameter>,
    request_body: Option<RequestBody>,
    request_stream_item_schema: Option<RefOr<Schema>>,
    response_status: &'static str,
    response_schema: Option<RefOr<Schema>>,
    response_stream_item_schema: Option<RefOr<Schema>>,
    summary: Option<String>,
    description: Option<String>,
    deprecated: bool,
    security_requirements: Option<Vec<HttpSecurityRequirement>>,
    security: Option<Vec<SecurityRequirement>>,
    response_content_type: String,
}

struct RouteTemplate {
    path: String,
    path_params: HashSet<String>,
    query_params: HashSet<String>,
}

fn render_op(
    op: &hir::OpDcl,
    interface_annotations: &[hir::Annotation],
    interface_name: &str,
    module_path: &[String],
) -> MethodInfo {
    validate_http_annotations(&format!("operation '{}'", op.ident), &op.annotations)
        .unwrap_or_else(|err| panic!("{err}"));
    let stream = http_stream_config(&op.annotations).unwrap_or_else(|err| panic!("{err}"));
    validate_http_stream_target(
        &op.ident,
        stream,
        HttpStreamTargetSupport {
            target: "openapi",
            supports_bidi: true,
            server_codec: HttpStreamCodec::Sse,
            client_codec: HttpStreamCodec::Ndjson,
            server_method: "GET",
            client_method: "POST",
            bidi_method: "GET",
        },
    )
    .unwrap_or_else(|err| panic!("{err}"));
    let is_server_stream = matches!(stream.kind, Some(HttpStreamKind::Server));
    let is_client_stream = matches!(stream.kind, Some(HttpStreamKind::Client));
    let is_bidi_stream = matches!(stream.kind, Some(HttpStreamKind::Bidi));
    let return_schema = match &op.ty {
        hir::OpTypeSpec::Void => None,
        hir::OpTypeSpec::TypeSpec(ty) => Some(schema_for_type(ty)),
    };

    let params = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);

    let default_method = if is_server_stream || is_bidi_stream {
        HttpMethod::Get
    } else {
        HttpMethod::Post
    };
    let (method, mut paths) = route_from_annotations(&op.annotations, default_method);
    validate_http_stream_method(
        &op.ident,
        stream.kind,
        method_name(method),
        HttpStreamTargetSupport {
            target: "openapi",
            supports_bidi: true,
            server_codec: HttpStreamCodec::Sse,
            client_codec: HttpStreamCodec::Ndjson,
            server_method: "GET",
            client_method: "POST",
            bidi_method: "GET",
        },
    )
    .unwrap_or_else(|err| panic!("{err}"));
    if paths.is_empty() {
        paths.push(auto_default_method_path(op, method));
    }
    let route_templates = paths
        .iter()
        .map(|value| parse_route_template(value))
        .collect::<Vec<_>>();
    let paths = route_templates
        .iter()
        .map(|value| openapi_path_template(&value.path))
        .collect::<Vec<_>>();
    validate_head_constraints(op, method);
    let path_param_sets = route_templates
        .iter()
        .map(|value| value.path_params.clone())
        .collect::<Vec<_>>();
    let all_path_param_names: HashSet<String> = path_param_sets
        .iter()
        .flat_map(|set| set.iter().cloned())
        .collect();
    let all_query_template_names: HashSet<String> = route_templates
        .iter()
        .flat_map(|value| value.query_params.iter().cloned())
        .collect();
    let default_source = if is_bidi_stream {
        ParamSource::Body
    } else {
        default_param_source(method)
    };

    let mut parameters = Vec::new();
    let mut body_props = Vec::new();
    let body_required = Vec::new();
    let mut output_fields = Vec::new();
    let mut path_binding_count = HashMap::<String, usize>::new();
    let mut query_binding_count = HashMap::<String, usize>::new();

    for param in params {
        let direction = param_direction(param.attr.as_ref());
        if matches!(direction, ParamDirection::Out) {
            continue;
        }
        if let Some(binding) = explicit_param_binding(param) {
            if matches!(binding.source, ParamSource::Path)
                && !all_path_param_names.contains(&binding.bound_name)
            {
                panic!(
                    "parameter '{}' is annotated with @path but '{}' is not present in any route template of method '{}'",
                    param.declarator.0, binding.bound_name, op.ident
                );
            }
        }
    }

    for param in params {
        let direction = param_direction(param.attr.as_ref());
        let raw_name = param.declarator.0.clone();
        let schema = schema_for_type(&param.ty);
        if matches!(direction, ParamDirection::Out | ParamDirection::InOut) {
            output_fields.push((raw_name.clone(), schema.clone()));
        }
        if matches!(direction, ParamDirection::Out) {
            continue;
        }
        let binding = explicit_param_binding(param);
        let (source, bound_name) = match binding {
            Some(binding) => (binding.source, binding.bound_name),
            None if all_path_param_names.contains(&raw_name) => {
                (ParamSource::Path, raw_name.clone())
            }
            None if all_query_template_names.contains(&raw_name) => {
                (ParamSource::Query, raw_name.clone())
            }
            None => (default_source, raw_name.clone()),
        };
        if matches!(source, ParamSource::Path)
            && !path_name_in_all_routes(&bound_name, &path_param_sets)
        {
            panic!(
                "parameter '{}' is bound to path variable '{}' but it is not present in every route template of method '{}'",
                param.declarator.0, bound_name, op.ident
            );
        }
        match source {
            ParamSource::Path => {
                *path_binding_count.entry(bound_name.clone()).or_insert(0) += 1;
                parameters.push(parameter_schema(
                    ParameterIn::Path,
                    &bound_name,
                    schema,
                    true,
                    doc_text(&param.annotations),
                ));
            }
            ParamSource::Query => {
                *query_binding_count.entry(bound_name.clone()).or_insert(0) += 1;
                parameters.push(parameter_schema(
                    ParameterIn::Query,
                    &bound_name,
                    schema,
                    false,
                    doc_text(&param.annotations),
                ));
            }
            ParamSource::Header => {
                parameters.push(parameter_schema(
                    ParameterIn::Header,
                    &bound_name,
                    schema,
                    false,
                    doc_text(&param.annotations),
                ));
            }
            ParamSource::Cookie => {
                parameters.push(parameter_schema(
                    ParameterIn::Cookie,
                    &bound_name,
                    schema,
                    false,
                    doc_text(&param.annotations),
                ));
            }
            ParamSource::Body => {
                let schema =
                    apply_schema_description(schema, doc_text(&param.annotations).as_deref());
                body_props.push((raw_name.clone(), schema));
            }
        }
    }
    for route_template in &route_templates {
        for query_param in &route_template.query_params {
            match query_binding_count.get(query_param).copied().unwrap_or(0) {
                0 => {
                    panic!(
                        "query template variable '{}' has no matching request-side query parameter in method '{}'",
                        query_param, op.ident
                    );
                }
                1 => {}
                _ => {
                    panic!(
                        "query template variable '{}' is bound by multiple request-side query parameters in method '{}'",
                        query_param, op.ident
                    );
                }
            }
        }
    }
    for route_params in &path_param_sets {
        for route_param in route_params {
            match path_binding_count.get(route_param).copied().unwrap_or(0) {
                0 => {
                    panic!(
                        "route template variable '{}' has no matching request-side path parameter in method '{}'",
                        route_param, op.ident
                    );
                }
                1 => {}
                _ => {
                    panic!(
                        "route template variable '{}' is bound by multiple request-side path parameters in method '{}'",
                        route_param, op.ident
                    );
                }
            }
        }
    }

    let output_count = usize::from(return_schema.is_some()) + output_fields.len();
    let (response_status, mut response_schema) = if matches!(method, HttpMethod::Head) {
        ("204", None)
    } else if output_count == 0 {
        ("204", None)
    } else if output_count == 1 {
        if let Some(schema) = return_schema {
            ("200", Some(schema))
        } else {
            let (_, schema) = output_fields.into_iter().next().unwrap();
            ("200", Some(schema))
        }
    } else {
        let mut object = ObjectBuilder::new().schema_type(Type::Object);
        if let Some(schema) = return_schema {
            object = object.property("return", schema).required("return");
        }
        for (name, schema) in output_fields {
            object = object.property(name.clone(), schema).required(name);
        }
        ("200", Some(RefOr::T(Schema::from(object))))
    };

    if is_bidi_stream {
        response_schema = response_schema.map(array_schema);
    }

    let mut request_schema = body_payload_schema(body_props, body_required);
    if is_bidi_stream {
        request_schema = request_schema.map(array_schema);
    }
    let request_content_type = if is_client_stream {
        "application/x-ndjson".to_string()
    } else {
        effective_media_type(interface_annotations, &op.annotations, "Consumes")
    };
    let response_content_type = if is_server_stream {
        "text/event-stream".to_string()
    } else {
        effective_media_type(interface_annotations, &op.annotations, "Produces")
    };
    let deprecated = effective_deprecated(interface_annotations, &op.annotations)
        .unwrap_or_else(|err| panic!("{err}"))
        .map(|value| value.deprecated)
        .unwrap_or(false);
    let security_requirements = effective_security(interface_annotations, &op.annotations)
        .unwrap_or_else(|err| panic!("{err}"))
        .map(|requirements| {
            let openapi = requirements
                .iter()
                .cloned()
                .map(openapi_security_requirement)
                .collect::<Vec<_>>();
            (requirements, openapi)
        });
    let (security_requirements, security) = match security_requirements {
        Some((requirements, openapi)) => (Some(requirements), Some(openapi)),
        None => (None, None),
    };
    let request_stream_item_schema = if is_client_stream {
        request_schema.clone()
    } else {
        None
    };
    let response_stream_item_schema = if is_server_stream {
        response_schema.clone()
    } else {
        None
    };
    MethodInfo {
        http_method: method_to_openapi(method),
        paths,
        operation_id: operation_id(module_path, interface_name, &op.ident),
        parameters,
        request_body: request_schema
            .map(|schema| request_body_schema(schema, &request_content_type)),
        request_stream_item_schema,
        response_status,
        response_schema,
        response_stream_item_schema,
        summary: doc_summary(&op.annotations),
        description: doc_text(&op.annotations),
        deprecated,
        security_requirements,
        security,
        response_content_type,
    }
}

fn render_attr(
    attr: &hir::AttrDcl,
    interface_annotations: &[hir::Annotation],
    interface_name: &str,
    module_path: &[String],
) -> Vec<MethodInfo> {
    validate_http_annotations(
        &format!("attribute in interface '{interface_name}'"),
        &attr.annotations,
    )
    .unwrap_or_else(|err| panic!("{err}"));
    let emit_watch = has_annotation(&attr.annotations, "server-stream");
    let summary = doc_summary(&attr.annotations);
    let description = doc_text(&attr.annotations);
    let deprecated = effective_deprecated(interface_annotations, &attr.annotations)
        .unwrap_or_else(|err| panic!("{err}"))
        .map(|value| value.deprecated)
        .unwrap_or(false);
    let security_requirements = effective_security(interface_annotations, &attr.annotations)
        .unwrap_or_else(|err| panic!("{err}"))
        .map(|requirements| {
            let openapi = requirements
                .iter()
                .cloned()
                .map(openapi_security_requirement)
                .collect::<Vec<_>>();
            (requirements, openapi)
        });
    let (security_requirements, security) = match security_requirements {
        Some((requirements, openapi)) => (Some(requirements), Some(openapi)),
        None => (None, None),
    };
    match &attr.decl {
        hir::AttrDclInner::ReadonlyAttrSpec(spec) => readonly_attr_names(spec)
            .into_iter()
            .flat_map(|raw_name| {
                let mut methods = vec![MethodInfo {
                    http_method: method_to_openapi(HttpMethod::Get),
                    paths: vec![default_path(module_path, interface_name, &raw_name)],
                    operation_id: operation_id(module_path, interface_name, &raw_name),
                    parameters: Vec::new(),
                    request_body: None,
                    request_stream_item_schema: None,
                    response_status: "200",
                    response_schema: Some(schema_for_type(&spec.ty)),
                    response_stream_item_schema: None,
                    summary: summary.clone(),
                    description: description.clone(),
                    deprecated,
                    security_requirements: security_requirements.clone(),
                    security: security.clone(),
                    response_content_type: "application/json".to_string(),
                }];
                if emit_watch {
                    let raw_watch = format!("watch_attribute_{raw_name}");
                    methods.push(MethodInfo {
                        http_method: method_to_openapi(HttpMethod::Get),
                        paths: vec![default_path(module_path, interface_name, &raw_watch)],
                        operation_id: operation_id(module_path, interface_name, &raw_watch),
                        parameters: Vec::new(),
                        request_body: None,
                        request_stream_item_schema: None,
                        response_status: "200",
                        response_schema: Some(schema_for_type(&spec.ty)),
                        response_stream_item_schema: Some(schema_for_type(&spec.ty)),
                        summary: summary.clone(),
                        description: description.clone(),
                        deprecated,
                        security_requirements: security_requirements.clone(),
                        security: security.clone(),
                        response_content_type: "text/event-stream".to_string(),
                    });
                }
                methods
            })
            .collect(),
        hir::AttrDclInner::AttrSpec(spec) => {
            let mut out = Vec::new();
            match &spec.declarator {
                hir::AttrDeclarator::SimpleDeclarator(list) => {
                    for decl in list {
                        let raw_name = decl.0.clone();
                        out.push(MethodInfo {
                            http_method: method_to_openapi(HttpMethod::Get),
                            paths: vec![default_path(module_path, interface_name, &raw_name)],
                            operation_id: operation_id(module_path, interface_name, &raw_name),
                            parameters: Vec::new(),
                            request_body: None,
                            request_stream_item_schema: None,
                            response_status: "200",
                            response_schema: Some(schema_for_type(&spec.ty)),
                            response_stream_item_schema: None,
                            summary: summary.clone(),
                            description: description.clone(),
                            deprecated,
                            security_requirements: security_requirements.clone(),
                            security: security.clone(),
                            response_content_type: "application/json".to_string(),
                        });
                        let raw_setter = format!("set_{raw_name}");
                        let props = vec![("value".to_string(), schema_for_type(&spec.ty))];
                        let required = vec!["value".to_string()];
                        out.push(MethodInfo {
                            http_method: method_to_openapi(HttpMethod::Post),
                            paths: vec![default_path(module_path, interface_name, &raw_setter)],
                            operation_id: operation_id(module_path, interface_name, &raw_setter),
                            parameters: Vec::new(),
                            request_body: body_schema(props, required, "application/json"),
                            request_stream_item_schema: None,
                            response_status: "204",
                            response_schema: None,
                            response_stream_item_schema: None,
                            summary: summary.clone(),
                            description: description.clone(),
                            deprecated,
                            security_requirements: security_requirements.clone(),
                            security: security.clone(),
                            response_content_type: "application/json".to_string(),
                        });
                        if emit_watch {
                            let raw_watch = format!("watch_attribute_{raw_name}");
                            out.push(MethodInfo {
                                http_method: method_to_openapi(HttpMethod::Get),
                                paths: vec![default_path(module_path, interface_name, &raw_watch)],
                                operation_id: operation_id(module_path, interface_name, &raw_watch),
                                parameters: Vec::new(),
                                request_body: None,
                                request_stream_item_schema: None,
                                response_status: "200",
                                response_schema: Some(schema_for_type(&spec.ty)),
                                response_stream_item_schema: Some(schema_for_type(&spec.ty)),
                                summary: summary.clone(),
                                description: description.clone(),
                                deprecated,
                                security_requirements: security_requirements.clone(),
                                security: security.clone(),
                                response_content_type: "text/event-stream".to_string(),
                            });
                        }
                    }
                }
                hir::AttrDeclarator::WithRaises { declarator, .. } => {
                    let raw_name = declarator.0.clone();
                    out.push(MethodInfo {
                        http_method: method_to_openapi(HttpMethod::Get),
                        paths: vec![default_path(module_path, interface_name, &raw_name)],
                        operation_id: operation_id(module_path, interface_name, &raw_name),
                        parameters: Vec::new(),
                        request_body: None,
                        request_stream_item_schema: None,
                        response_status: "200",
                        response_schema: Some(schema_for_type(&spec.ty)),
                        response_stream_item_schema: None,
                        summary: summary.clone(),
                        description: description.clone(),
                        deprecated,
                        security_requirements: security_requirements.clone(),
                        security: security.clone(),
                        response_content_type: "application/json".to_string(),
                    });
                    let raw_setter = format!("set_{raw_name}");
                    let props = vec![("value".to_string(), schema_for_type(&spec.ty))];
                    let required = vec!["value".to_string()];
                    out.push(MethodInfo {
                        http_method: method_to_openapi(HttpMethod::Post),
                        paths: vec![default_path(module_path, interface_name, &raw_setter)],
                        operation_id: operation_id(module_path, interface_name, &raw_setter),
                        parameters: Vec::new(),
                        request_body: body_schema(props, required, "application/json"),
                        request_stream_item_schema: None,
                        response_status: "204",
                        response_schema: None,
                        response_stream_item_schema: None,
                        summary: summary.clone(),
                        description: description.clone(),
                        deprecated,
                        security_requirements: security_requirements.clone(),
                        security: security.clone(),
                        response_content_type: "application/json".to_string(),
                    });
                    if emit_watch {
                        let raw_watch = format!("watch_attribute_{raw_name}");
                        out.push(MethodInfo {
                            http_method: method_to_openapi(HttpMethod::Get),
                            paths: vec![default_path(module_path, interface_name, &raw_watch)],
                            operation_id: operation_id(module_path, interface_name, &raw_watch),
                            parameters: Vec::new(),
                            request_body: None,
                            request_stream_item_schema: None,
                            response_status: "200",
                            response_schema: Some(schema_for_type(&spec.ty)),
                            response_stream_item_schema: Some(schema_for_type(&spec.ty)),
                            summary: summary.clone(),
                            description: description.clone(),
                            deprecated,
                            security_requirements: security_requirements.clone(),
                            security: security.clone(),
                            response_content_type: "text/event-stream".to_string(),
                        });
                    }
                }
            }
            out
        }
    }
}

fn body_schema(
    props: Vec<(String, RefOr<Schema>)>,
    required: Vec<String>,
    content_type: &str,
) -> Option<RequestBody> {
    let schema = body_payload_schema(props, required)?;
    Some(request_body_schema(schema, content_type))
}

fn body_payload_schema(
    props: Vec<(String, RefOr<Schema>)>,
    required: Vec<String>,
) -> Option<RefOr<Schema>> {
    if props.is_empty() {
        return None;
    }
    if props.len() == 1 {
        let (_, schema) = props.into_iter().next()?;
        return Some(schema);
    }
    let mut object = ObjectBuilder::new().schema_type(Type::Object);
    for (name, schema) in props {
        object = object.property(name.clone(), schema);
    }
    for name in required {
        object = object.required(name);
    }
    Some(RefOr::T(Schema::from(object)))
}

fn request_body_schema(schema: RefOr<Schema>, content_type: &str) -> RequestBody {
    let mut request_body = RequestBody::new();
    request_body
        .content
        .insert(content_type.to_string(), Content::new(Some(schema)));
    request_body
}

fn request_stream_content_type(request_body: &Option<RequestBody>) -> String {
    request_body
        .as_ref()
        .and_then(|body| body.content.keys().next().cloned())
        .unwrap_or_else(|| panic!("stream request body is missing content"))
}

fn patch_openapi_stream_content(doc: &mut Value, patch: &OpenApiStreamPatch) {
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

fn array_schema(items: RefOr<Schema>) -> RefOr<Schema> {
    RefOr::T(Schema::from(ArrayBuilder::new().items(items)))
}

fn parameter_schema(
    location: ParameterIn,
    name: &str,
    schema: RefOr<Schema>,
    required: bool,
    description: Option<String>,
) -> utoipa::openapi::path::Parameter {
    let required = if required {
        Required::True
    } else {
        Required::False
    };
    let mut builder = ParameterBuilder::new()
        .name(name)
        .parameter_in(location)
        .required(required)
        .schema(Some(schema));
    if let Some(description) = description {
        builder = builder.description(Some(description));
    }
    builder.build()
}

fn schema_for_struct(members: &[hir::Member]) -> RefOr<Schema> {
    let mut object = ObjectBuilder::new().schema_type(Type::Object);
    for member in members {
        let rename = field_rename(&member.annotations);
        let optional = member.is_optional();
        let doc = doc_text(&member.annotations);
        for decl in &member.ident {
            let name = rename.clone().unwrap_or_else(|| declarator_name(decl));
            let schema =
                apply_schema_description(schema_for_decl(&member.ty, decl), doc.as_deref());
            object = object.property(name.clone(), schema);
            if !optional {
                object = object.required(name);
            }
        }
    }
    RefOr::T(Schema::from(object))
}

fn schema_for_union(def: &hir::UnionDef) -> RefOr<Schema> {
    let mut variants = Vec::new();
    for case in &def.case {
        let decl = &case.element.value;
        let name = declarator_name(decl);
        let schema = apply_schema_description(
            schema_for_element(&case.element.ty, decl),
            doc_text(&case.element.annotations).as_deref(),
        );
        let object = ObjectBuilder::new()
            .schema_type(Type::Object)
            .property(name.clone(), schema)
            .required(name);
        variants.push(RefOr::T(Schema::from(object)));
    }
    let mut one_of = OneOf::new();
    one_of.items = variants;
    RefOr::T(Schema::from(one_of))
}

fn schema_for_element(ty: &hir::ElementSpecTy, decl: &hir::Declarator) -> RefOr<Schema> {
    match ty {
        hir::ElementSpecTy::TypeSpec(spec) => schema_for_decl(spec, decl),
        hir::ElementSpecTy::ConstrTypeDcl(constr) => schema_for_constr_type(constr, &[]),
    }
}

fn schema_for_decl(ty: &hir::TypeSpec, decl: &hir::Declarator) -> RefOr<Schema> {
    let mut schema = schema_for_type(ty);
    if let hir::Declarator::ArrayDeclarator(array) = decl {
        for len in &array.len {
            let size = xidl_parser::hir::const_expr_to_i64(&len.0);
            let mut array_schema = ArrayBuilder::new().items(schema);
            if let Some(size) = size {
                if size >= 0 {
                    let size = size as usize;
                    array_schema = array_schema.min_items(Some(size)).max_items(Some(size));
                }
            }
            schema = RefOr::T(Schema::from(array_schema));
        }
    }
    schema
}

fn apply_schema_description(mut schema: RefOr<Schema>, doc: Option<&str>) -> RefOr<Schema> {
    let Some(doc) = doc.filter(|value| !value.is_empty()) else {
        return schema;
    };
    match &mut schema {
        RefOr::T(Schema::Object(object)) => {
            object.description = Some(doc.to_string());
        }
        RefOr::T(Schema::Array(array)) => {
            array.description = Some(doc.to_string());
        }
        RefOr::T(Schema::OneOf(one_of)) => {
            one_of.description = Some(doc.to_string());
        }
        _ => {}
    }
    schema
}

fn doc_text(annotations: &[hir::Annotation]) -> Option<String> {
    let lines = doc_lines_from_annotations(annotations);
    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

fn doc_summary(annotations: &[hir::Annotation]) -> Option<String> {
    let lines = doc_lines_from_annotations(annotations);
    lines.first().cloned()
}

fn effective_deprecated(
    interface_annotations: &[hir::Annotation],
    method_annotations: &[hir::Annotation],
) -> Result<Option<DeprecatedInfo>, String> {
    if let Some(info) = deprecated_info(method_annotations)? {
        return Ok(Some(info));
    }
    deprecated_info(interface_annotations)
}

fn openapi_security_requirement(requirement: HttpSecurityRequirement) -> SecurityRequirement {
    match requirement {
        HttpSecurityRequirement::HttpBasic => {
            SecurityRequirement::new("http-basic", Vec::<String>::new())
        }
        HttpSecurityRequirement::HttpBearer => {
            SecurityRequirement::new("http-bearer", Vec::<String>::new())
        }
        HttpSecurityRequirement::ApiKey { location, name } => {
            SecurityRequirement::new(api_key_scheme_name(&location, &name), Vec::<String>::new())
        }
        HttpSecurityRequirement::OAuth2 { scopes } => SecurityRequirement::new("oauth2", scopes),
    }
}

fn register_security_schemes(
    store: &mut BTreeMap<String, SecurityScheme>,
    security: &[HttpSecurityRequirement],
) {
    for requirement in security {
        match requirement {
            HttpSecurityRequirement::HttpBasic => {
                store
                    .entry("http-basic".to_string())
                    .or_insert_with(|| SecurityScheme::Http(Http::new(HttpAuthScheme::Basic)));
            }
            HttpSecurityRequirement::HttpBearer => {
                store
                    .entry("http-bearer".to_string())
                    .or_insert_with(|| SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)));
            }
            HttpSecurityRequirement::ApiKey { location, name } => {
                let key = api_key_scheme_name(location, name);
                store.entry(key).or_insert_with(|| match location {
                    HttpApiKeyLocation::Header => {
                        SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new(name.clone())))
                    }
                    HttpApiKeyLocation::Query => {
                        SecurityScheme::ApiKey(ApiKey::Query(ApiKeyValue::new(name.clone())))
                    }
                    HttpApiKeyLocation::Cookie => {
                        SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new(name.clone())))
                    }
                });
            }
            HttpSecurityRequirement::OAuth2 { scopes } => {
                store.entry("oauth2".to_string()).or_insert_with(|| {
                    let scopes = scopes
                        .iter()
                        .map(|scope| (scope.clone(), scope.clone()))
                        .collect::<Vec<_>>();
                    SecurityScheme::OAuth2(OAuth2::new([
                        utoipa::openapi::security::Flow::ClientCredentials(
                            utoipa::openapi::security::ClientCredentials::new(
                                "https://example.invalid/token",
                                Scopes::from_iter(scopes),
                            ),
                        ),
                    ]))
                });
            }
        }
    }
}

fn api_key_scheme_name(location: &HttpApiKeyLocation, name: &str) -> String {
    let location = match location {
        HttpApiKeyLocation::Header => "header",
        HttpApiKeyLocation::Query => "query",
        HttpApiKeyLocation::Cookie => "cookie",
    };
    format!("api-key-{location}-{}", name.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{self, AssertUnwindSafe};
    use xidl_parser::hir;

    fn parse_spec(source: &str) -> hir::Specification {
        let typed = xidl_parser::parser::parser_text(source).expect("parse typed ast");
        hir::Specification::from_typed_ast_with_properties(typed, HashMap::new())
    }

    fn doc_annotation(text: &str) -> hir::Annotation {
        hir::Annotation::Builtin {
            name: "doc".to_string(),
            params: Some(hir::AnnotationParams::Raw(format!("\"{}\"", text))),
        }
    }

    #[test]
    fn schema_for_struct_applies_doc_to_fields() {
        let member = hir::Member {
            annotations: vec![doc_annotation("field doc")],
            ty: hir::TypeSpec::SimpleTypeSpec(hir::SimpleTypeSpec::IntegerType(
                hir::IntegerType::I32,
            )),
            ident: vec![hir::Declarator::SimpleDeclarator(hir::SimpleDeclarator(
                "value".to_string(),
            ))],
            default: None,
            field_id: None,
        };
        let schema = schema_for_struct(&[member]);
        let RefOr::T(Schema::Object(object)) = schema else {
            panic!("expected object schema");
        };
        let Some(prop) = object.properties.get("value") else {
            panic!("missing value property");
        };
        let RefOr::T(Schema::Object(prop_obj)) = prop else {
            panic!("expected object property schema");
        };
        assert_eq!(prop_obj.description.as_deref(), Some("field doc"));
    }

    #[test]
    fn render_openapi_json_uses_32_and_item_schema_for_streams() {
        let spec = parse_spec(
            r#"
            interface StreamApi {
              @server-stream
              @stream-codec("sse")
              string watch();

              @client-stream
              @stream-codec("ndjson")
              string upload(
                in string file_id,
                in sequence<octet> chunk
              );
            };
            "#,
        );
        let doc = render_openapi_json(&spec).expect("render openapi json");
        assert_eq!(
            doc.get("openapi"),
            Some(&Value::String("3.2.0".to_string()))
        );

        let server_content =
            &doc["paths"]["/watch"]["get"]["responses"]["200"]["content"]["text/event-stream"];
        assert!(server_content.get("itemSchema").is_some());
        assert!(server_content.get("schema").is_none());

        let client_content =
            &doc["paths"]["/upload"]["post"]["requestBody"]["content"]["application/x-ndjson"];
        assert!(client_content.get("itemSchema").is_some());
        assert!(client_content.get("schema").is_none());
    }

    fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
        if let Some(message) = payload.downcast_ref::<&'static str>() {
            (*message).to_string()
        } else if let Some(message) = payload.downcast_ref::<String>() {
            message.clone()
        } else {
            "unknown panic payload".to_string()
        }
    }

    #[test]
    fn render_openapi_json_rejects_invalid_stream_codec() {
        let spec = parse_spec(
            r#"
            interface StreamApi {
              @server-stream
              @stream-codec("yaml")
              string watch();
            };
            "#,
        );
        let payload = panic::catch_unwind(AssertUnwindSafe(|| render_openapi_json(&spec)))
            .expect_err("invalid stream codec should panic");
        let message = panic_message(payload);
        assert!(message.contains("unsupported @stream-codec value"));
    }

    #[test]
    fn render_openapi_json_rejects_invalid_stream_method() {
        let spec = parse_spec(
            r#"
            interface StreamApi {
              @server-stream
              @stream-codec("sse")
              @post(path = "/watch")
              string watch();
            };
            "#,
        );
        let payload = panic::catch_unwind(AssertUnwindSafe(|| render_openapi_json(&spec)))
            .expect_err("invalid stream method should panic");
        let message = panic_message(payload);
        assert!(message.contains("@server-stream method 'watch' must use GET"));
    }

    #[test]
    fn render_openapi_json_rejects_duplicate_security_annotations() {
        let spec = parse_spec(
            r#"
            interface SecurityApi {
              @http-basic
              @http-basic
              @get(path = "/reports")
              string list_reports();
            };
            "#,
        );
        let payload = panic::catch_unwind(AssertUnwindSafe(|| render_openapi_json(&spec)))
            .expect_err("duplicate security annotations should panic");
        let message = panic_message(payload);
        assert!(message.contains("duplicate @http-basic annotation"));
    }

    #[test]
    fn render_openapi_json_rejects_conflicting_no_security_annotations() {
        let spec = parse_spec(
            r#"
            interface SecurityApi {
              @no-security
              @http-bearer
              @get(path = "/reports")
              string list_reports();
            };
            "#,
        );
        let payload = panic::catch_unwind(AssertUnwindSafe(|| render_openapi_json(&spec)))
            .expect_err("conflicting security annotations should panic");
        let message = panic_message(payload);
        assert!(
            message.contains("@no-security cannot be combined with other security annotations")
        );
    }

    #[test]
    fn render_openapi_json_rejects_conflicting_param_sources() {
        let spec = parse_spec(
            r#"
            interface HttpApi {
              @get(path = "/users/{id}")
              string get_user(
                @path("id") @query("user_id") string id
              );
            };
            "#,
        );
        let payload = panic::catch_unwind(AssertUnwindSafe(|| render_openapi_json(&spec)))
            .expect_err("conflicting parameter sources should panic");
        let message = panic_message(payload);
        assert!(message.contains("conflicting source annotations"));
    }

    #[test]
    fn render_openapi_json_rejects_missing_query_template_binding() {
        let spec = parse_spec(
            r#"
            interface HttpApi {
              @get(path = "/users/{id}{?lang,region}")
              string get_user(
                @path("id") string id,
                @query("lang") string lang
              );
            };
            "#,
        );
        let payload = panic::catch_unwind(AssertUnwindSafe(|| render_openapi_json(&spec)))
            .expect_err("missing query template binding should panic");
        let message = panic_message(payload);
        assert!(message.contains(
            "query template variable 'region' has no matching request-side query parameter"
        ));
    }

    #[test]
    fn render_openapi_json_rejects_duplicate_route_bindings() {
        let spec = parse_spec(
            r#"
            interface HttpApi {
              @get(path = "/users/{id}")
              string get_user(@path("id") string id);

              @get(path = "/users/{id}")
              string fetch_user(@path("id") string id);
            };
            "#,
        );
        let payload = panic::catch_unwind(AssertUnwindSafe(|| render_openapi_json(&spec)))
            .expect_err("duplicate route binding should panic");
        let message = panic_message(payload);
        assert!(message.contains("duplicate HTTP route binding"));
    }

    #[test]
    fn render_openapi_json_rejects_additional_invalid_security_annotations() {
        let duplicate_bearer = parse_spec(
            r#"
            interface SecurityApi {
              @http-bearer
              @http-bearer
              @get(path = "/reports")
              string list_reports();
            };
            "#,
        );
        let payload =
            panic::catch_unwind(AssertUnwindSafe(|| render_openapi_json(&duplicate_bearer)))
                .expect_err("duplicate bearer should panic");
        let message = panic_message(payload);
        assert!(message.contains("duplicate @http-bearer annotation"));

        let missing_name = parse_spec(
            r#"
            interface SecurityApi {
              @api-key(in = "header")
              @get(path = "/reports")
              string list_reports();
            };
            "#,
        );
        let payload = panic::catch_unwind(AssertUnwindSafe(|| render_openapi_json(&missing_name)))
            .expect_err("api key missing name should panic");
        let message = panic_message(payload);
        assert!(message.contains("@api-key requires non-empty name=..."));

        let invalid_location = parse_spec(
            r#"
            interface SecurityApi {
              @api-key(in = "body", name = "auth")
              @get(path = "/reports")
              string list_reports();
            };
            "#,
        );
        let payload =
            panic::catch_unwind(AssertUnwindSafe(|| render_openapi_json(&invalid_location)))
                .expect_err("api key invalid location should panic");
        let message = panic_message(payload);
        assert!(message.contains("must be one of header|query|cookie"));
    }

    #[test]
    fn render_openapi_json_rejects_additional_invalid_stream_shapes() {
        let mutually_exclusive = parse_spec(
            r#"
            interface StreamApi {
              @server-stream
              @client-stream
              @stream-codec("ndjson")
              @post(path = "/events")
              string exchange(string payload);
            };
            "#,
        );
        let payload = panic::catch_unwind(AssertUnwindSafe(|| {
            render_openapi_json(&mutually_exclusive)
        }))
        .expect_err("mutually exclusive stream annotations should panic");
        let message = panic_message(payload);
        assert!(message.contains("mutually exclusive"));

        let client_sse = parse_spec(
            r#"
            interface StreamApi {
              @client-stream
              @stream-codec("sse")
              @post(path = "/upload")
              string upload(sequence<octet> chunk);
            };
            "#,
        );
        let payload = panic::catch_unwind(AssertUnwindSafe(|| render_openapi_json(&client_sse)))
            .expect_err("client stream sse should panic");
        let message = panic_message(payload);
        assert!(
            message.contains("supports only NDJSON for @client-stream methods")
                || message.contains("@stream-codec(\"sse\") requires @server-stream")
        );
    }
}

fn schema_for_type(ty: &hir::TypeSpec) -> RefOr<Schema> {
    match ty {
        hir::TypeSpec::SimpleTypeSpec(simple) => match simple {
            hir::SimpleTypeSpec::IntegerType(value) => integer_schema(value),
            hir::SimpleTypeSpec::FloatingPtType => RefOr::T(Schema::from(
                ObjectBuilder::new()
                    .schema_type(Type::Number)
                    .format(Some(SchemaFormat::KnownFormat(KnownFormat::Double))),
            )),
            hir::SimpleTypeSpec::CharType | hir::SimpleTypeSpec::WideCharType => {
                RefOr::T(Schema::from(ObjectBuilder::new().schema_type(Type::String)))
            }
            hir::SimpleTypeSpec::Boolean => RefOr::T(Schema::from(
                ObjectBuilder::new().schema_type(Type::Boolean),
            )),
            hir::SimpleTypeSpec::AnyType
            | hir::SimpleTypeSpec::ObjectType
            | hir::SimpleTypeSpec::ValueBaseType => RefOr::T(Schema::from(ObjectBuilder::new())),
            hir::SimpleTypeSpec::ScopedName(value) => schema_ref(&scoped_name_ref(value)),
        },
        hir::TypeSpec::TemplateTypeSpec(template) => match template {
            hir::TemplateTypeSpec::SequenceType(seq) => {
                let mut schema = ArrayBuilder::new().items(schema_for_type(&seq.ty));
                if let Some(len) = &seq.len {
                    if let Some(size) = xidl_parser::hir::const_expr_to_i64(&len.0) {
                        if size >= 0 {
                            let size = size as usize;
                            schema = schema.min_items(Some(size)).max_items(Some(size));
                        }
                    }
                }
                RefOr::T(Schema::from(schema))
            }
            hir::TemplateTypeSpec::StringType(_) | hir::TemplateTypeSpec::WideStringType(_) => {
                RefOr::T(Schema::from(ObjectBuilder::new().schema_type(Type::String)))
            }
            hir::TemplateTypeSpec::FixedPtType(_) => RefOr::T(Schema::from(
                ObjectBuilder::new()
                    .schema_type(Type::Number)
                    .format(Some(SchemaFormat::KnownFormat(KnownFormat::Double))),
            )),
            hir::TemplateTypeSpec::MapType(map) => RefOr::T(Schema::from(
                ObjectBuilder::new()
                    .schema_type(Type::Object)
                    .additional_properties(Some(schema_for_type(&map.value))),
            )),
            hir::TemplateTypeSpec::TemplateType(_) => {
                RefOr::T(Schema::from(ObjectBuilder::new().schema_type(Type::Object)))
            }
        },
    }
}

fn integer_schema(value: &hir::IntegerType) -> RefOr<Schema> {
    let mut object = ObjectBuilder::new().schema_type(Type::Integer);
    match value {
        hir::IntegerType::U64 => {
            object = object
                .format(Some(SchemaFormat::KnownFormat(KnownFormat::Int64)))
                .minimum(Some(0));
        }
        hir::IntegerType::U32
        | hir::IntegerType::U16
        | hir::IntegerType::U8
        | hir::IntegerType::UChar => {
            object = object
                .format(Some(SchemaFormat::KnownFormat(KnownFormat::Int32)))
                .minimum(Some(0));
        }
        hir::IntegerType::I64 => {
            object = object.format(Some(SchemaFormat::KnownFormat(KnownFormat::Int64)));
        }
        _ => {
            object = object.format(Some(SchemaFormat::KnownFormat(KnownFormat::Int32)));
        }
    }
    RefOr::T(Schema::from(object))
}

fn schema_ref(name: &str) -> RefOr<Schema> {
    RefOr::Ref(Ref::from_schema_name(name))
}

fn scoped_name_ref(value: &hir::ScopedName) -> String {
    value.name.join(".")
}

fn declarator_name(decl: &hir::Declarator) -> String {
    match decl {
        hir::Declarator::SimpleDeclarator(simple) => simple.0.clone(),
        hir::Declarator::ArrayDeclarator(array) => array.ident.clone(),
    }
}

fn scoped_name(module_path: &[String], ident: &str) -> String {
    if module_path.is_empty() {
        ident.to_string()
    } else {
        let mut parts = module_path.to_vec();
        parts.push(ident.to_string());
        parts.join(".")
    }
}

fn operation_id(module_path: &[String], interface_name: &str, method_name: &str) -> String {
    let mut parts = module_path.to_vec();
    parts.push(interface_name.to_string());
    parts.push(method_name.to_string());
    parts.join(".")
}

fn default_path(module_path: &[String], interface_name: &str, method_name: &str) -> String {
    let mut parts = module_path.to_vec();
    parts.push(interface_name.to_string());
    parts.push(method_name.to_string());
    format!("/{}", parts.join("/"))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ParamSource {
    Path,
    Query,
    Header,
    Cookie,
    Body,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ParamDirection {
    In,
    Out,
    InOut,
}

fn route_from_annotations(
    annotations: &[hir::Annotation],
    default_method: HttpMethod,
) -> (HttpMethod, Vec<String>) {
    let mut verb_method = None;
    let mut paths = Vec::new();

    for annotation in annotations {
        let Some(name) = annotation_name(annotation) else {
            continue;
        };
        if let Some(method) = method_from_annotation(annotation) {
            if let Some(prev) = verb_method {
                if prev != method {
                    panic!("more than one HTTP verb annotation is not allowed on a method");
                }
            }
            verb_method = Some(method);
            if let Some(params) = annotation_params(annotation) {
                let params = normalize_params(params);
                if let Some(path) = params.get("path") {
                    paths.push(normalize_path(path));
                }
            }
            continue;
        }
        if name.eq_ignore_ascii_case("path") {
            if let Some(params) = annotation_params(annotation) {
                let params = normalize_params(params);
                if let Some(path) = params.get("value").or_else(|| params.get("path")) {
                    paths.push(normalize_path(path));
                }
            }
        }
    }

    let mut dedup = HashSet::new();
    paths.retain(|path| dedup.insert(path.clone()));
    (verb_method.unwrap_or(default_method), paths)
}

fn method_from_annotation(annotation: &hir::Annotation) -> Option<HttpMethod> {
    let name = annotation_name(annotation)?;
    match name.to_ascii_lowercase().as_str() {
        "get" => Some(HttpMethod::Get),
        "post" => Some(HttpMethod::Post),
        "put" => Some(HttpMethod::Put),
        "patch" => Some(HttpMethod::Patch),
        "delete" => Some(HttpMethod::Delete),
        "head" => Some(HttpMethod::Head),
        "options" => Some(HttpMethod::Options),
        _ => None,
    }
}

fn annotation_name(annotation: &hir::Annotation) -> Option<&str> {
    match annotation {
        hir::Annotation::Builtin { name, .. } => Some(name.as_str()),
        hir::Annotation::ScopedName { name, .. } => name.name.last().map(|value| value.as_str()),
        _ => None,
    }
}

fn annotation_params(annotation: &hir::Annotation) -> Option<&hir::AnnotationParams> {
    match annotation {
        hir::Annotation::Builtin { params, .. } => params.as_ref(),
        hir::Annotation::ScopedName { params, .. } => params.as_ref(),
        _ => None,
    }
}

fn has_annotation(annotations: &[hir::Annotation], target: &str) -> bool {
    annotations.iter().any(|annotation| {
        annotation_name(annotation)
            .map(|name| name.eq_ignore_ascii_case(target))
            .unwrap_or(false)
    })
}

fn field_rename(annotations: &[hir::Annotation]) -> Option<String> {
    for annotation in annotations {
        let Some(name) = annotation_name(annotation) else {
            continue;
        };
        if !name.eq_ignore_ascii_case("name") {
            continue;
        }
        let value = annotation_params(annotation)
            .map(normalize_params)
            .and_then(|params| {
                params
                    .get("value")
                    .cloned()
                    .or_else(|| params.get("name").cloned())
            });
        if value.is_some() {
            return value;
        }
    }
    None
}

fn normalize_params(params: &hir::AnnotationParams) -> HashMap<String, String> {
    let mut out = HashMap::new();
    match params {
        hir::AnnotationParams::Raw(value) => {
            for (key, value) in parse_raw_params(value) {
                out.insert(key.to_ascii_lowercase(), value);
            }
        }
        hir::AnnotationParams::Params(values) => {
            for value in values {
                let raw = value
                    .value
                    .as_ref()
                    .map(render_const_expr)
                    .unwrap_or_default();
                out.insert(
                    value.ident.to_ascii_lowercase(),
                    trim_quotes(&raw).unwrap_or(raw),
                );
            }
        }
        hir::AnnotationParams::ConstExpr(expr) => {
            let rendered = render_const_expr(expr);
            out.insert(
                "value".to_string(),
                trim_quotes(&rendered).unwrap_or(rendered),
            );
        }
    }
    out
}

fn parse_raw_params(raw: &str) -> Vec<(String, String)> {
    let mut parts = Vec::new();
    let mut buf = String::new();
    let mut quote = None;
    let mut escaped = false;

    for ch in raw.chars() {
        if escaped {
            buf.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' && quote.is_some() {
            escaped = true;
            buf.push(ch);
            continue;
        }
        match ch {
            '\'' | '"' => {
                if quote == Some(ch) {
                    quote = None;
                } else if quote.is_none() {
                    quote = Some(ch);
                }
                buf.push(ch);
            }
            ',' if quote.is_none() => {
                let item = buf.trim();
                if !item.is_empty() {
                    parts.push(item.to_string());
                }
                buf.clear();
            }
            _ => buf.push(ch),
        }
    }

    let item = buf.trim();
    if !item.is_empty() {
        parts.push(item.to_string());
    }

    let mut out = Vec::new();
    for part in parts {
        if let Some((key, value)) = part.split_once('=') {
            let value = trim_quotes(value.trim()).unwrap_or_else(|| value.trim().to_string());
            out.push((key.trim().to_string(), unescape_param_value(&value)));
        }
    }
    out
}

fn unescape_param_value(value: &str) -> String {
    let mut out = String::new();
    let mut escaped = false;
    for ch in value.chars() {
        if escaped {
            out.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        out.push(ch);
    }
    out
}

fn trim_quotes(value: &str) -> Option<String> {
    let value = value.trim();
    if value.len() >= 2 {
        let first = value.chars().next().unwrap();
        let last = value.chars().last().unwrap();
        if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
            return Some(value[1..value.len() - 1].to_string());
        }
    }
    None
}

fn render_const_expr(expr: &hir::ConstExpr) -> String {
    crate::generate::render_const_expr(
        expr,
        &crate::generate::rust::util::rust_scoped_name,
        &crate::generate::rust::util::rust_literal,
    )
}

fn parse_path_params(path: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut buf = String::new();
    let mut in_param = false;

    for ch in path.chars() {
        match ch {
            '{' if !in_param => {
                in_param = true;
                buf.clear();
            }
            '}' if in_param => {
                if !buf.is_empty() {
                    out.insert(strip_path_param_prefix(&buf));
                }
                in_param = false;
            }
            _ => {
                if in_param {
                    buf.push(ch);
                }
            }
        }
    }

    out
}

fn strip_path_param_prefix(value: &str) -> String {
    value.strip_prefix('*').unwrap_or(value).to_string()
}

fn openapi_path_template(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    let mut in_param = false;
    let mut buf = String::new();
    for ch in path.chars() {
        match ch {
            '{' if !in_param => {
                in_param = true;
                buf.clear();
                out.push('{');
            }
            '}' if in_param => {
                out.push_str(buf.strip_prefix('*').unwrap_or(&buf));
                out.push('}');
                in_param = false;
            }
            _ if in_param => buf.push(ch),
            _ => out.push(ch),
        }
    }
    out
}

fn validate_route_template(path: &str) {
    let (path, _) = split_query_template(path);
    let mut start = 0usize;
    let mut catch_all_count = 0usize;
    while let Some(open_rel) = path[start..].find('{') {
        let open = start + open_rel;
        let close = path[open + 1..]
            .find('}')
            .map(|value| open + 1 + value)
            .unwrap_or_else(|| panic!("route template has unmatched '{{' in '{path}'"));
        let token = &path[open + 1..close];
        let is_catch_all = token.starts_with('*');
        let name = token.strip_prefix('*').unwrap_or(token);
        assert!(
            !name.is_empty(),
            "route template has empty path variable in '{path}'"
        );
        if is_catch_all {
            catch_all_count += 1;
            assert!(
                catch_all_count <= 1,
                "route template contains more than one catch-all variable: '{path}'"
            );
            assert!(
                close + 1 == path.len(),
                "catch-all variable must be at the end of route template: '{path}'"
            );
        }
        start = close + 1;
    }
}

fn split_query_template(path: &str) -> (String, HashSet<String>) {
    let mut query_params = HashSet::new();
    if let Some(pos) = path.find("{?") {
        assert!(
            path.ends_with('}'),
            "query template must terminate with '}}' in route '{path}'"
        );
        let tail = &path[pos + 2..path.len() - 1];
        assert!(
            !tail.trim().is_empty(),
            "query template must include at least one variable in route '{path}'"
        );
        for name in tail.split(',').map(|value| value.trim()) {
            assert!(
                !name.is_empty(),
                "query template contains empty variable name in route '{path}'"
            );
            query_params.insert(name.to_string());
        }
        (path[..pos].to_string(), query_params)
    } else {
        (path.to_string(), query_params)
    }
}

fn parse_route_template(path: &str) -> RouteTemplate {
    validate_route_template(path);
    let (path, query_params) = split_query_template(path);
    let normalized = normalize_path(&path);
    let path_params = parse_path_params(&normalized);
    RouteTemplate {
        path: normalized,
        path_params,
        query_params,
    }
}

fn normalize_path(path: &str) -> String {
    let path = path.trim();
    let with_leading = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    let mut collapsed = String::with_capacity(with_leading.len());
    let mut prev_slash = false;
    for ch in with_leading.chars() {
        if ch == '/' {
            if !prev_slash {
                collapsed.push(ch);
            }
            prev_slash = true;
        } else {
            collapsed.push(ch);
            prev_slash = false;
        }
    }
    if collapsed.len() > 1 && collapsed.ends_with('/') {
        collapsed.pop();
    }
    if collapsed.is_empty() {
        "/".to_string()
    } else {
        collapsed
    }
}

fn path_name_in_all_routes(name: &str, route_sets: &[HashSet<String>]) -> bool {
    route_sets.iter().all(|set| set.contains(name))
}

fn validate_head_constraints(op: &hir::OpDcl, method: HttpMethod) {
    if !matches!(method, HttpMethod::Head) {
        return;
    }
    if !matches!(op.ty, hir::OpTypeSpec::Void) {
        panic!("HEAD method '{}' must return void", op.ident);
    }
    let params = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);
    for param in params {
        if matches!(
            param_direction(param.attr.as_ref()),
            ParamDirection::Out | ParamDirection::InOut
        ) {
            panic!(
                "HEAD method '{}' cannot contain out/inout parameter '{}'",
                op.ident, param.declarator.0
            );
        }
    }
}

fn default_param_source(method: HttpMethod) -> ParamSource {
    match method {
        HttpMethod::Get | HttpMethod::Delete | HttpMethod::Head | HttpMethod::Options => {
            ParamSource::Query
        }
        HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch => ParamSource::Body,
    }
}

fn param_direction(attr: Option<&hir::ParamAttribute>) -> ParamDirection {
    match attr.map(|value| value.0.as_str()) {
        Some("out") => ParamDirection::Out,
        Some("inout") => ParamDirection::InOut,
        _ => ParamDirection::In,
    }
}

struct SourceBinding {
    source: ParamSource,
    bound_name: String,
}

fn explicit_param_binding(param: &hir::ParamDcl) -> Option<SourceBinding> {
    let mut found = None;
    for annotation in &param.annotations {
        let Some(name) = annotation_name(annotation) else {
            continue;
        };
        let current = if name.eq_ignore_ascii_case("path") {
            Some(ParamSource::Path)
        } else if name.eq_ignore_ascii_case("query") {
            Some(ParamSource::Query)
        } else if name.eq_ignore_ascii_case("header") {
            Some(ParamSource::Header)
        } else if name.eq_ignore_ascii_case("cookie") {
            Some(ParamSource::Cookie)
        } else {
            None
        };
        let Some(current) = current else {
            continue;
        };
        let bound_name = annotation_params(annotation)
            .map(normalize_params)
            .and_then(|params| params.get("value").cloned())
            .unwrap_or_else(|| param.declarator.0.clone());
        if matches!(current, ParamSource::Header) {
            validate_header_name(&bound_name, &param.declarator.0);
        }
        if matches!(current, ParamSource::Cookie) {
            validate_cookie_name(&bound_name, &param.declarator.0);
        }
        match found {
            None => {
                found = Some(SourceBinding {
                    source: current,
                    bound_name,
                })
            }
            Some(ref prev) if prev.source == current && prev.bound_name == bound_name => {}
            Some(_) => {
                panic!(
                    "parameter '{}' has conflicting source annotations (@path/@query/@header/@cookie)",
                    param.declarator.0
                );
            }
        }
    }
    found
}

fn validate_header_name(bound_name: &str, param_name: &str) {
    if bound_name.is_empty() {
        panic!("parameter '{}' has empty @header name", param_name);
    }
    if bound_name.starts_with(':') {
        panic!(
            "parameter '{}' uses reserved pseudo-header name '{}'",
            param_name, bound_name
        );
    }
}

fn validate_cookie_name(bound_name: &str, param_name: &str) {
    if bound_name.is_empty() {
        panic!("parameter '{}' has empty @cookie name", param_name);
    }
    if bound_name
        .chars()
        .any(|ch| ch.is_ascii_whitespace() || ch == ';' || ch == '=')
    {
        panic!(
            "parameter '{}' has invalid @cookie name '{}'",
            param_name, bound_name
        );
    }
}

fn auto_default_method_path(op: &hir::OpDcl, method: HttpMethod) -> String {
    let params = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);
    let default_source = default_param_source(method);
    let mut path = normalize_path(&op.ident);
    for param in params {
        if matches!(param_direction(param.attr.as_ref()), ParamDirection::Out) {
            continue;
        }
        let binding = explicit_param_binding(param);
        let (source, bound_name) = match binding {
            Some(binding) => (binding.source, binding.bound_name),
            None => (default_source, param.declarator.0.clone()),
        };
        if matches!(source, ParamSource::Path) {
            path.push('/');
            path.push('{');
            path.push_str(&bound_name);
            path.push('}');
        }
    }
    path
}

fn method_to_openapi(method: HttpMethod) -> OpenApiHttpMethod {
    match method {
        HttpMethod::Get => OpenApiHttpMethod::Get,
        HttpMethod::Post => OpenApiHttpMethod::Post,
        HttpMethod::Put => OpenApiHttpMethod::Put,
        HttpMethod::Patch => OpenApiHttpMethod::Patch,
        HttpMethod::Delete => OpenApiHttpMethod::Delete,
        HttpMethod::Head => OpenApiHttpMethod::Head,
        HttpMethod::Options => OpenApiHttpMethod::Options,
    }
}

fn openapi_method_name(method: &OpenApiHttpMethod) -> &'static str {
    match method {
        OpenApiHttpMethod::Get => "get",
        OpenApiHttpMethod::Post => "post",
        OpenApiHttpMethod::Put => "put",
        OpenApiHttpMethod::Patch => "patch",
        OpenApiHttpMethod::Delete => "delete",
        OpenApiHttpMethod::Head => "head",
        OpenApiHttpMethod::Options => "options",
        _ => "unknown",
    }
}

fn method_name(method: HttpMethod) -> &'static str {
    match method {
        HttpMethod::Get => "GET",
        HttpMethod::Post => "POST",
        HttpMethod::Put => "PUT",
        HttpMethod::Patch => "PATCH",
        HttpMethod::Delete => "DELETE",
        HttpMethod::Head => "HEAD",
        HttpMethod::Options => "OPTIONS",
    }
}

fn error_schema_ref() -> RefOr<Schema> {
    schema_ref("Error")
}

fn readonly_attr_names(spec: &hir::ReadonlyAttrSpec) -> Vec<String> {
    match &spec.declarator {
        hir::ReadonlyAttrDeclarator::SimpleDeclarator(decl) => vec![decl.0.clone()],
        hir::ReadonlyAttrDeclarator::RaisesExpr(_) => Vec::new(),
    }
}

fn schema_for_constr_type(constr: &hir::ConstrTypeDcl, module_path: &[String]) -> RefOr<Schema> {
    match constr {
        hir::ConstrTypeDcl::StructDcl(def) => {
            let name = scoped_name(module_path, &def.ident);
            schema_ref(&name)
        }
        hir::ConstrTypeDcl::EnumDcl(def) => {
            let name = scoped_name(module_path, &def.ident);
            schema_ref(&name)
        }
        hir::ConstrTypeDcl::UnionDef(def) => {
            let name = scoped_name(module_path, &def.ident);
            schema_ref(&name)
        }
        hir::ConstrTypeDcl::BitsetDcl(def) => {
            let name = scoped_name(module_path, &def.ident);
            schema_ref(&name)
        }
        hir::ConstrTypeDcl::BitmaskDcl(def) => {
            let name = scoped_name(module_path, &def.ident);
            schema_ref(&name)
        }
        hir::ConstrTypeDcl::StructForwardDcl(def) => {
            let name = scoped_name(module_path, &def.ident);
            schema_ref(&name)
        }
        hir::ConstrTypeDcl::UnionForwardDcl(def) => {
            let name = scoped_name(module_path, &def.ident);
            schema_ref(&name)
        }
    }
}
