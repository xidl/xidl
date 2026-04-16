use crate::openapi::path::{
    HttpMethod as OpenApiHttpMethod, OperationBuilder, ParameterBuilder, ParameterIn, PathItem,
    PathsBuilder,
};
use crate::openapi::request_body::RequestBody;
use crate::openapi::response::ResponseBuilder;
use crate::openapi::schema::{
    ArrayBuilder, KnownFormat, ObjectBuilder, OneOf, Schema, SchemaFormat, Type,
};
use crate::openapi::security::{
    ApiKey, ApiKeyValue, Http, HttpAuthScheme, OAuth2, Scopes, SecurityRequirement, SecurityScheme,
};
use crate::openapi::server::Server;
use crate::openapi::{
    Content, InfoBuilder, OpenApi, OpenApiBuilder, Ref, RefOr, Required, ResponsesBuilder,
};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::mem;
use xidl_parser::hir;
use xidl_parser::hir::{ParserProperties, Specification};

use crate::generate::http_hir::{
    HttpHirDocument, HttpMethod as HttpHirMethod, HttpOperation,
    HttpParamSource as HttpHirParamSource,
    semantics::{
        DeprecatedInfo, HttpApiKeyLocation, HttpSecurityRequirement, HttpStreamCodec,
        HttpStreamKind, annotation_name, annotation_params, normalize_annotation_params,
    },
};
use crate::generate::utils::doc_lines_from_annotations;
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
        props: ParserProperties,
    ) -> Result<Vec<Artifact>, xidl_jsonrpc::Error> {
        let http_hir =
            HttpHirDocument::from_props(&props).map_err(|err| xidl_jsonrpc::Error::Rpc {
                code: xidl_jsonrpc::ErrorCode::ServerError,
                message: err.to_string(),
                data: None,
            })?;
        let openapi = render_openapi_json(&hir, &http_hir)?;
        let content = serde_json::to_string_pretty(&openapi)?;
        Ok(vec![Artifact::new_file(ArtifactFile {
            path: "openapi.json".to_string(),
            content,
        })])
    }
}

fn render_openapi_json(
    spec: &hir::Specification,
    http_hir: &HttpHirDocument,
) -> Result<Value, serde_json::Error> {
    let ctx = render_openapi(spec, http_hir);
    let version = select_openapi_version(&ctx);
    let mut value = serde_json::to_value(ctx.document)?;
    if let Some(openapi) = value.get_mut("openapi") {
        *openapi = Value::String(version.to_string());
    }
    for patch in ctx.stream_patches {
        patch_openapi_stream_content(&mut value, &patch);
    }
    Ok(value)
}

fn select_openapi_version(ctx: &RenderedOpenApi) -> &'static str {
    if ctx.stream_patches.is_empty() {
        "3.1.0"
    } else {
        // Stream itemSchema requires OpenAPI 3.2.0.
        "3.2.0"
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
    fn collect_spec(
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
                    ok_response = ok_response
                        .content(&response_content_type, Content::new(Some(schema.clone())));
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

fn render_http_operation(
    op: &HttpOperation,
    module_path: &[String],
    interface_name: &str,
) -> MethodInfo {
    match op.stream.kind {
        Some(HttpStreamKind::Server) if op.stream.codec != HttpStreamCodec::Sse => {
            panic!(
                "openapi currently supports only SSE for @server_stream methods: '{}'",
                op.name
            );
        }
        Some(HttpStreamKind::Client) if op.stream.codec != HttpStreamCodec::Ndjson => {
            panic!(
                "openapi currently supports only NDJSON for @client_stream methods: '{}'",
                op.name
            );
        }
        _ => {}
    }

    let mut parameters = Vec::new();
    let mut body_props = Vec::new();
    let body_required = Vec::new();
    let mut output_fields = Vec::new();

    for param in &op.request_params {
        let raw_name = param.name.clone();
        let schema = schema_for_type(&param.ty);
        match param.source {
            HttpHirParamSource::Path => {
                parameters.push(parameter_schema(
                    ParameterIn::Path,
                    &param.wire_name,
                    schema,
                    true,
                    None,
                ));
            }
            HttpHirParamSource::Query => {
                parameters.push(parameter_schema(
                    ParameterIn::Query,
                    &param.wire_name,
                    schema,
                    false,
                    None,
                ));
            }
            HttpHirParamSource::Header => parameters.push(parameter_schema(
                ParameterIn::Header,
                &param.wire_name,
                schema,
                false,
                None,
            )),
            HttpHirParamSource::Cookie => parameters.push(parameter_schema(
                ParameterIn::Cookie,
                &param.wire_name,
                schema,
                false,
                None,
            )),
            HttpHirParamSource::Body => body_props.push((raw_name, schema)),
        }
    }

    for param in &op.response_params {
        output_fields.push((param.wire_name.clone(), schema_for_type(&param.ty)));
    }
    let return_schema = op.return_type.as_ref().map(schema_for_type);
    let output_count = usize::from(return_schema.is_some()) + output_fields.len();
    let is_head = matches!(op.method, HttpHirMethod::Head);
    let (response_status, mut response_schema) = if is_head || output_count == 0 {
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

    if matches!(op.stream.kind, Some(HttpStreamKind::Bidi)) {
        response_schema = response_schema.map(array_schema);
    }
    let mut request_schema = body_payload_schema(body_props, body_required);
    if matches!(op.stream.kind, Some(HttpStreamKind::Bidi)) {
        request_schema = request_schema.map(array_schema);
    }

    let request_content_type = if matches!(op.stream.kind, Some(HttpStreamKind::Client)) {
        "application/x-ndjson".to_string()
    } else {
        op.request_content_type.clone()
    };
    let response_content_type = if matches!(op.stream.kind, Some(HttpStreamKind::Server)) {
        "text/event-stream".to_string()
    } else {
        op.response_content_type.clone()
    };
    let security_requirements = op.security.as_ref().map(|profile| {
        let requirements = profile.requirements.clone();
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
    let request_stream_item_schema = if matches!(op.stream.kind, Some(HttpStreamKind::Client)) {
        request_schema.clone()
    } else {
        None
    };
    let response_stream_item_schema = if matches!(op.stream.kind, Some(HttpStreamKind::Server)) {
        response_schema.clone()
    } else {
        None
    };

    MethodInfo {
        http_method: method_to_openapi(http_method_from_hir(op.method)),
        paths: op
            .routes
            .iter()
            .map(|route| openapi_path_template(&route.path))
            .collect(),
        operation_id: operation_id(module_path, interface_name, &op.name),
        parameters,
        request_body: request_schema
            .map(|schema| request_body_schema(schema, &request_content_type)),
        request_stream_item_schema,
        response_status,
        response_schema,
        response_stream_item_schema,
        summary: None,
        description: None,
        deprecated: op
            .deprecated
            .as_ref()
            .map(|value| value.deprecated)
            .unwrap_or(false),
        deprecated_info: op.deprecated.clone(),
        security_requirements,
        security,
        response_content_type,
    }
}

fn http_method_from_hir(method: HttpHirMethod) -> HttpMethod {
    match method {
        HttpHirMethod::Get => HttpMethod::Get,
        HttpHirMethod::Post => HttpMethod::Post,
        HttpHirMethod::Put => HttpMethod::Put,
        HttpHirMethod::Patch => HttpMethod::Patch,
        HttpHirMethod::Delete => HttpMethod::Delete,
        HttpHirMethod::Head => HttpMethod::Head,
        HttpHirMethod::Options => HttpMethod::Options,
    }
}

struct MethodInfo {
    http_method: OpenApiHttpMethod,
    paths: Vec<String>,
    operation_id: String,
    parameters: Vec<crate::openapi::path::Parameter>,
    request_body: Option<RequestBody>,
    request_stream_item_schema: Option<RefOr<Schema>>,
    response_status: &'static str,
    response_schema: Option<RefOr<Schema>>,
    response_stream_item_schema: Option<RefOr<Schema>>,
    summary: Option<String>,
    description: Option<String>,
    deprecated: bool,
    deprecated_info: Option<DeprecatedInfo>,
    security_requirements: Option<Vec<HttpSecurityRequirement>>,
    security: Option<Vec<SecurityRequirement>>,
    response_content_type: String,
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
) -> crate::openapi::path::Parameter {
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

fn openapi_security_requirement(requirement: HttpSecurityRequirement) -> SecurityRequirement {
    match requirement {
        HttpSecurityRequirement::HttpBasic => {
            SecurityRequirement::new("http_basic", Vec::<String>::new())
        }
        HttpSecurityRequirement::HttpBearer => {
            SecurityRequirement::new("http_bearer", Vec::<String>::new())
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
                    .entry("http_basic".to_string())
                    .or_insert_with(|| SecurityScheme::Http(Http::new(HttpAuthScheme::Basic)));
            }
            HttpSecurityRequirement::HttpBearer => {
                store
                    .entry("http_bearer".to_string())
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
                        crate::openapi::security::Flow::ClientCredentials(
                            crate::openapi::security::ClientCredentials::new(
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
    format!("api_key-{location}-{}", name.to_ascii_lowercase())
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

    fn render_openapi_json_from_spec(
        spec: &hir::Specification,
    ) -> Result<Value, serde_json::Error> {
        let http_hir = crate::generate::http_hir::project(spec).expect("project http hir");
        render_openapi_json(spec, &http_hir)
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
    fn render_openapi_json_defaults_to_31_without_streams() {
        let spec = parse_spec(
            r#"
            interface HelloApi {
              string hello();
            };
            "#,
        );
        let doc = render_openapi_json_from_spec(&spec).expect("render openapi json");
        assert_eq!(
            doc.get("openapi"),
            Some(&Value::String("3.1.0".to_string()))
        );
    }

    #[test]
    fn render_openapi_json_uses_32_and_item_schema_for_streams() {
        let spec = parse_spec(
            r#"
            interface StreamApi {
              @server_stream
              @stream_codec("sse")
              string watch();

              @client_stream
              @stream_codec("ndjson")
              string upload(
                in string file_id,
                in sequence<octet> chunk
              );
            };
            "#,
        );
        let doc = render_openapi_json_from_spec(&spec).expect("render openapi json");
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
              @server_stream
              @stream_codec("yaml")
              string watch();
            };
            "#,
        );
        let payload =
            panic::catch_unwind(AssertUnwindSafe(|| render_openapi_json_from_spec(&spec)))
                .expect_err("invalid stream codec should panic");
        let message = panic_message(payload);
        assert!(message.contains("unsupported @stream_codec value"));
    }

    #[test]
    fn render_openapi_json_rejects_invalid_stream_method() {
        let spec = parse_spec(
            r#"
            interface StreamApi {
              @server_stream
              @stream_codec("sse")
              @post(path = "/watch")
              string watch();
            };
            "#,
        );
        let payload =
            panic::catch_unwind(AssertUnwindSafe(|| render_openapi_json_from_spec(&spec)))
                .expect_err("invalid stream method should panic");
        let message = panic_message(payload);
        assert!(message.contains("@server_stream method 'watch' must use GET"));
    }

    #[test]
    fn render_openapi_json_rejects_duplicate_security_annotations() {
        let spec = parse_spec(
            r#"
            interface SecurityApi {
              @http_basic
              @http_basic
              @get(path = "/reports")
              string list_reports();
            };
            "#,
        );
        let payload =
            panic::catch_unwind(AssertUnwindSafe(|| render_openapi_json_from_spec(&spec)))
                .expect_err("duplicate security annotations should panic");
        let message = panic_message(payload);
        assert!(message.contains("duplicate @http_basic annotation"));
    }

    #[test]
    fn render_openapi_json_rejects_conflicting_no_security_annotations() {
        let spec = parse_spec(
            r#"
            interface SecurityApi {
              @no_security
              @http_bearer
              @get(path = "/reports")
              string list_reports();
            };
            "#,
        );
        let payload =
            panic::catch_unwind(AssertUnwindSafe(|| render_openapi_json_from_spec(&spec)))
                .expect_err("conflicting security annotations should panic");
        let message = panic_message(payload);
        assert!(
            message.contains("@no_security cannot be combined with other security annotations")
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
        let payload =
            panic::catch_unwind(AssertUnwindSafe(|| render_openapi_json_from_spec(&spec)))
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
        let payload =
            panic::catch_unwind(AssertUnwindSafe(|| render_openapi_json_from_spec(&spec)))
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
        let payload =
            panic::catch_unwind(AssertUnwindSafe(|| render_openapi_json_from_spec(&spec)))
                .expect_err("duplicate route binding should panic");
        let message = panic_message(payload);
        assert!(message.contains("duplicate HTTP route binding"));
    }

    #[test]
    fn render_openapi_json_rejects_additional_invalid_security_annotations() {
        let duplicate_bearer = parse_spec(
            r#"
            interface SecurityApi {
              @http_bearer
              @http_bearer
              @get(path = "/reports")
              string list_reports();
            };
            "#,
        );
        let payload = panic::catch_unwind(AssertUnwindSafe(|| {
            render_openapi_json_from_spec(&duplicate_bearer)
        }))
        .expect_err("duplicate bearer should panic");
        let message = panic_message(payload);
        assert!(message.contains("duplicate @http_bearer annotation"));

        let missing_name = parse_spec(
            r#"
            interface SecurityApi {
              @api_key(in = "header")
              @get(path = "/reports")
              string list_reports();
            };
            "#,
        );
        let payload = panic::catch_unwind(AssertUnwindSafe(|| {
            render_openapi_json_from_spec(&missing_name)
        }))
        .expect_err("api key missing name should panic");
        let message = panic_message(payload);
        assert!(message.contains("@api_key requires non-empty name=..."));

        let invalid_location = parse_spec(
            r#"
            interface SecurityApi {
              @api_key(in = "body", name = "auth")
              @get(path = "/reports")
              string list_reports();
            };
            "#,
        );
        let payload = panic::catch_unwind(AssertUnwindSafe(|| {
            render_openapi_json_from_spec(&invalid_location)
        }))
        .expect_err("api key invalid location should panic");
        let message = panic_message(payload);
        assert!(message.contains("must be one of header|query|cookie"));
    }

    #[test]
    fn render_openapi_json_rejects_additional_invalid_stream_shapes() {
        let mutually_exclusive = parse_spec(
            r#"
            interface StreamApi {
              @server_stream
              @client_stream
              @stream_codec("ndjson")
              @post(path = "/events")
              string exchange(string payload);
            };
            "#,
        );
        let payload = panic::catch_unwind(AssertUnwindSafe(|| {
            render_openapi_json_from_spec(&mutually_exclusive)
        }))
        .expect_err("mutually exclusive stream annotations should panic");
        let message = panic_message(payload);
        assert!(message.contains("mutually exclusive"));

        let client_sse = parse_spec(
            r#"
            interface StreamApi {
              @client_stream
              @stream_codec("sse")
              @post(path = "/upload")
              string upload(sequence<octet> chunk);
            };
            "#,
        );
        let payload = panic::catch_unwind(AssertUnwindSafe(|| {
            render_openapi_json_from_spec(&client_sse)
        }))
        .expect_err("client stream sse should panic");
        let message = panic_message(payload);
        assert!(
            message.contains("supports only NDJSON for @client_stream methods")
                || message.contains("requires @server_stream")
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

fn field_rename(annotations: &[hir::Annotation]) -> Option<String> {
    for annotation in annotations {
        let Some(name) = annotation_name(annotation) else {
            continue;
        };
        if !name.eq_ignore_ascii_case("name") {
            continue;
        }
        let value = annotation_params(annotation)
            .map(normalize_annotation_params)
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

fn error_schema_ref() -> RefOr<Schema> {
    schema_ref("Error")
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
