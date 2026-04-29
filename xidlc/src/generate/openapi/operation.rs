use crate::generate::http_hir::{
    HttpMethod as HttpHirMethod, HttpOperation, HttpParamKind as HttpHirParamKind,
    semantics::{
        DeprecatedInfo, HttpApiKeyLocation, HttpSecurityRequirement, HttpStreamCodec,
        HttpStreamKind,
    },
};
use crate::generate::openapi::schema::{
    ParameterOptions, array_schema, parameter_schema, schema_for_type,
};
use crate::generate::openapi::utils::{openapi_path_template, operation_id};
use crate::openapi::path::HttpMethod as OpenApiHttpMethod;
use crate::openapi::request_body::RequestBody;
use crate::openapi::security::{
    ApiKey, ApiKeyValue, Http, HttpAuthScheme, OAuth2, Scopes, SecurityRequirement, SecurityScheme,
};
use crate::openapi::{Content, RefOr};
use std::collections::BTreeMap;

/// Information about a rendered HTTP operation.
pub struct MethodInfo {
    pub http_method: OpenApiHttpMethod,
    pub paths: Vec<String>,
    pub operation_id: String,
    pub parameters: Vec<crate::openapi::path::Parameter>,
    pub request_body: Option<RequestBody>,
    pub request_stream_item_schema: Option<RefOr<crate::openapi::schema::Schema>>,
    pub response_status: &'static str,
    pub response_schema: Option<RefOr<crate::openapi::schema::Schema>>,
    pub response_stream_item_schema: Option<RefOr<crate::openapi::schema::Schema>>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub deprecated: bool,
    pub deprecated_info: Option<DeprecatedInfo>,
    pub security_requirements: Option<Vec<HttpSecurityRequirement>>,
    pub security: Option<Vec<SecurityRequirement>>,
    pub response_content_type: String,
}

/// Renders an HTTP operation to `MethodInfo`.
pub fn render_http_operation(
    op: &HttpOperation,
    module_path: &[String],
    interface_name: &str,
) -> MethodInfo {
    validate_stream_config(op);

    let mut parameters = Vec::new();
    let mut body_props = Vec::new();

    for param in &op.request_params {
        let raw_name = param.name.clone();
        let schema = schema_for_type(&param.ty);
        match param.kind {
            HttpHirParamKind::Path => {
                parameters.push(parameter_schema(ParameterOptions {
                    location: crate::openapi::path::ParameterIn::Path,
                    name: param.wire_name.clone(),
                    schema,
                    required: true,
                    description: None,
                }));
            }
            HttpHirParamKind::Query => {
                parameters.push(parameter_schema(ParameterOptions {
                    location: crate::openapi::path::ParameterIn::Query,
                    name: param.wire_name.clone(),
                    schema,
                    required: false,
                    description: None,
                }));
            }
            HttpHirParamKind::Header => parameters.push(parameter_schema(ParameterOptions {
                location: crate::openapi::path::ParameterIn::Header,
                name: param.wire_name.clone(),
                schema,
                required: false,
                description: None,
            })),
            HttpHirParamKind::Cookie => parameters.push(parameter_schema(ParameterOptions {
                location: crate::openapi::path::ParameterIn::Cookie,
                name: param.wire_name.clone(),
                schema,
                required: false,
                description: None,
            })),
            HttpHirParamKind::Body => body_props.push((raw_name, schema)),
        }
    }

    let mut output_fields = Vec::new();
    for param in &op.response_params {
        output_fields.push((param.wire_name.clone(), schema_for_type(&param.ty)));
    }
    let return_schema = op.return_type.as_ref().map(schema_for_type);
    let output_count = usize::from(return_schema.is_some()) + output_fields.len();
    let is_head = matches!(op.method, HttpHirMethod::Head);

    let (response_status, mut response_schema) =
        assemble_response_schema(is_head, output_count, return_schema, output_fields);

    if matches!(op.stream.kind, Some(HttpStreamKind::Bidi)) {
        response_schema = response_schema.map(array_schema);
    }
    let mut request_schema = body_payload_schema(body_props, Vec::new());
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

    let (security_requirements, security) = assemble_security(op);

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
        http_method: method_to_openapi(op.method),
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

fn validate_stream_config(op: &HttpOperation) {
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
}

fn assemble_response_schema(
    is_head: bool,
    output_count: usize,
    return_schema: Option<RefOr<crate::openapi::schema::Schema>>,
    output_fields: Vec<(String, RefOr<crate::openapi::schema::Schema>)>,
) -> (&'static str, Option<RefOr<crate::openapi::schema::Schema>>) {
    if is_head || output_count == 0 {
        ("204", None)
    } else if output_count == 1 {
        if let Some(schema) = return_schema {
            ("200", Some(schema))
        } else {
            let (_, schema) = output_fields.into_iter().next().unwrap();
            ("200", Some(schema))
        }
    } else {
        let mut object = crate::openapi::schema::ObjectBuilder::new()
            .schema_type(crate::openapi::schema::Type::Object);
        if let Some(schema) = return_schema {
            object = object.property("return", schema).required("return");
        }
        for (name, schema) in output_fields {
            object = object.property(name.clone(), schema).required(name);
        }
        (
            "200",
            Some(RefOr::T(crate::openapi::schema::Schema::from(object))),
        )
    }
}

fn assemble_security(
    op: &HttpOperation,
) -> (
    Option<Vec<HttpSecurityRequirement>>,
    Option<Vec<SecurityRequirement>>,
) {
    op.security
        .as_ref()
        .map(|profile| {
            let requirements = profile.requirements.clone();
            let openapi = requirements
                .iter()
                .cloned()
                .map(openapi_security_requirement)
                .collect::<Vec<_>>();
            (Some(requirements), Some(openapi))
        })
        .unwrap_or((None, None))
}

fn method_to_openapi(method: HttpHirMethod) -> OpenApiHttpMethod {
    match method {
        HttpHirMethod::Get => OpenApiHttpMethod::Get,
        HttpHirMethod::Post => OpenApiHttpMethod::Post,
        HttpHirMethod::Put => OpenApiHttpMethod::Put,
        HttpHirMethod::Patch => OpenApiHttpMethod::Patch,
        HttpHirMethod::Delete => OpenApiHttpMethod::Delete,
        HttpHirMethod::Head => OpenApiHttpMethod::Head,
        HttpHirMethod::Options => OpenApiHttpMethod::Options,
    }
}

fn body_payload_schema(
    props: Vec<(String, RefOr<crate::openapi::schema::Schema>)>,
    required: Vec<String>,
) -> Option<RefOr<crate::openapi::schema::Schema>> {
    if props.is_empty() {
        return None;
    }
    if props.len() == 1 {
        let (_, schema) = props.into_iter().next()?;
        return Some(schema);
    }
    let mut object = crate::openapi::schema::ObjectBuilder::new()
        .schema_type(crate::openapi::schema::Type::Object);
    for (name, schema) in props {
        object = object.property(name.clone(), schema);
    }
    for name in required {
        object = object.required(name);
    }
    Some(RefOr::T(crate::openapi::schema::Schema::from(object)))
}

fn request_body_schema(
    schema: RefOr<crate::openapi::schema::Schema>,
    content_type: &str,
) -> RequestBody {
    let mut request_body = RequestBody::new();
    request_body
        .content
        .insert(content_type.to_string(), Content::new(Some(schema)));
    request_body
}

pub fn openapi_security_requirement(requirement: HttpSecurityRequirement) -> SecurityRequirement {
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

pub fn register_security_schemes(
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
    let loc_str = match location {
        HttpApiKeyLocation::Header => "header",
        HttpApiKeyLocation::Query => "query",
        HttpApiKeyLocation::Cookie => "cookie",
    };
    format!("api_key-{loc_str}-{}", name.to_ascii_lowercase())
}
