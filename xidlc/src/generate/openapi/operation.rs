use super::naming::{method_to_openapi, openapi_path_template, operation_id};
use super::schema::{
    apply_deprecation_note, array_schema, parameter_schema, request_body_schema, schema_for_type,
    schema_ref,
};
use super::security::openapi_security_requirement;
use crate::openapi::path::{HttpMethod as OpenApiHttpMethod, Parameter, ParameterIn};
use crate::openapi::request_body::RequestBody;
use crate::openapi::schema::{ObjectBuilder, Schema, Type};
use crate::openapi::{RefOr, security::SecurityRequirement};
use xidl_parser::rest_hir::{
    HttpException, HttpOperation, HttpRequestBodyShape, HttpResponseBodyShape,
    semantics::{HttpSecurityRequirement, HttpStreamCodec, HttpStreamKind},
};

pub(crate) struct MethodInfo {
    pub(crate) http_method: OpenApiHttpMethod,
    pub(crate) paths: Vec<String>,
    pub(crate) operation_id: String,
    pub(crate) parameters: Vec<Parameter>,
    pub(crate) request_body: Option<RequestBody>,
    pub(crate) request_stream_item_schema: Option<RefOr<Schema>>,
    pub(crate) response_status: String,
    pub(crate) response_schema: Option<RefOr<Schema>>,
    pub(crate) response_stream_item_schema: Option<RefOr<Schema>>,
    pub(crate) summary: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) deprecated: bool,
    pub(crate) security_requirements: Option<Vec<HttpSecurityRequirement>>,
    pub(crate) security: Option<Vec<SecurityRequirement>>,
    pub(crate) response_content_type: String,
    pub(crate) tag: String,
    pub(crate) is_websocket: bool,
    pub(crate) websocket_subprotocol: Option<String>,
    /// `raises(...)` exceptions projected as typed error responses.
    pub(crate) raises: Vec<ExceptionResponseInfo>,
    /// `@http` union representations for the success response.
    pub(crate) representations: Vec<RepresentationInfo>,
}

/// One negotiated success representation of an `@http` union response.
pub(crate) struct RepresentationInfo {
    pub(crate) content_type: String,
    pub(crate) schema: RefOr<Schema>,
}

/// One declared exception rendered as an OpenAPI response.
pub(crate) struct ExceptionResponseInfo {
    pub(crate) status: String,
    pub(crate) ident: String,
    /// Response headers: `(wire_name, schema)` from `@header` members.
    pub(crate) headers: Vec<(String, RefOr<Schema>)>,
    /// `$ref` to the exception schema when it has body members.
    pub(crate) schema: Option<RefOr<Schema>>,
}

pub(crate) fn render_http_operation(
    op: &HttpOperation,
    module_path: &[String],
    interface_name: &str,
    exceptions: &[HttpException],
) -> MethodInfo {
    validate_stream_contract(op);

    let mut parameters = Vec::new();

    for binding in &op.http.request.path {
        parameters.push(parameter_schema(
            ParameterIn::Path,
            &binding.wire_name,
            schema_for_type(&binding.ty),
            true,
            None,
        ));
    }
    for binding in &op.http.request.query {
        parameters.push(parameter_schema(
            ParameterIn::Query,
            &binding.wire_name,
            schema_for_type(&binding.ty),
            !binding.optional,
            None,
        ));
    }
    for binding in &op.http.request.header {
        parameters.push(parameter_schema(
            ParameterIn::Header,
            &binding.wire_name,
            schema_for_type(&binding.ty),
            !binding.optional,
            None,
        ));
    }
    for binding in &op.http.request.cookie {
        parameters.push(parameter_schema(
            ParameterIn::Cookie,
            &binding.wire_name,
            schema_for_type(&binding.ty),
            !binding.optional,
            None,
        ));
    }

    let request_schema = match &op.http.request.body.shape {
        HttpRequestBodyShape::Empty => None,
        HttpRequestBodyShape::SingleValue { ty, .. }
            if xidl_parser::rest_hir::is_byte_sequence(ty) =>
        {
            // Raw byte payloads: binary string, no element schema.
            Some(RefOr::T(Schema::from(
                ObjectBuilder::new().schema_type(Type::String),
            )))
        }
        HttpRequestBodyShape::SingleValue { ty, .. } => Some(schema_for_type(ty)),
        HttpRequestBodyShape::Object { fields } => {
            let mut object = ObjectBuilder::new().schema_type(Type::Object);
            for field in fields {
                object = object.property(field.field_name.clone(), schema_for_type(&field.ty));
                if !field.optional {
                    object = object.required(&field.field_name);
                }
            }
            Some(RefOr::T(Schema::from(object)))
        }
        HttpRequestBodyShape::Stream { item_ty, .. } => Some(schema_for_type(item_ty)),
    };

    let response_schema = match &op.http.response.body.shape {
        HttpResponseBodyShape::Empty => None,
        HttpResponseBodyShape::ReturnOnly { ty } => Some(schema_for_type(ty)),
        HttpResponseBodyShape::SingleValue { ty, .. } => Some(schema_for_type(ty)),
        HttpResponseBodyShape::Object { fields } => {
            let mut object = ObjectBuilder::new().schema_type(Type::Object);
            for field in fields {
                object = object.property(field.field_name.clone(), schema_for_type(&field.ty));
                object = object.required(&field.field_name);
            }
            Some(RefOr::T(Schema::from(object)))
        }
        HttpResponseBodyShape::Stream { item_ty, .. } => Some(schema_for_type(item_ty)),
    };

    let mut final_request_schema = request_schema.clone();
    let mut final_response_schema = response_schema.clone();
    if matches!(op.meta.stream.kind, Some(HttpStreamKind::Bidi)) {
        final_request_schema = final_request_schema.map(array_schema);
        final_response_schema = final_response_schema.map(array_schema);
    }

    let (security_requirements, security) = op
        .meta
        .security
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
        .unwrap_or((None, None));

    let request_content_type = op
        .http
        .request
        .body
        .content_type
        .clone()
        .unwrap_or_else(|| "application/json".to_string());
    let response_content_type = op
        .http
        .response
        .body
        .content_type
        .clone()
        .unwrap_or_else(|| "application/json".to_string());

    let response_status = op.http.response.status.clone();

    MethodInfo {
        http_method: method_to_openapi(op.meta.method),
        paths: op
            .meta
            .routes
            .iter()
            .map(|route| openapi_path_template(&route.path))
            .collect(),
        operation_id: operation_id(module_path, interface_name, &op.meta.name),
        parameters,
        request_body: final_request_schema
            .map(|schema| request_body_schema(schema, &request_content_type)),
        request_stream_item_schema: matches!(op.meta.stream.kind, Some(HttpStreamKind::Client))
            .then_some(request_schema)
            .flatten(),
        response_status,
        response_schema: final_response_schema,
        response_stream_item_schema: matches!(op.meta.stream.kind, Some(HttpStreamKind::Server))
            .then_some(response_schema)
            .flatten(),
        summary: None,
        description: apply_deprecation_note(None, op.meta.deprecated.as_ref()),
        deprecated: op
            .meta
            .deprecated
            .as_ref()
            .map(|value| value.deprecated)
            .unwrap_or(false),
        security_requirements,
        security,
        response_content_type,
        tag: interface_name.to_string(),
        is_websocket: matches!(op.meta.stream.kind, Some(HttpStreamKind::Bidi)),
        websocket_subprotocol: op
            .meta
            .websocket
            .as_ref()
            .and_then(|cfg| cfg.subprotocol.clone()),
        representations: op
            .http
            .response
            .representations
            .iter()
            .map(|repr| {
                let schema = if repr.is_byte {
                    RefOr::T(Schema::from(
                        ObjectBuilder::new().schema_type(Type::String).format(Some(
                            crate::openapi::schema::SchemaFormat::KnownFormat(
                                crate::openapi::schema::KnownFormat::Binary,
                            ),
                        )),
                    ))
                } else {
                    schema_for_type(&repr.ty)
                };
                RepresentationInfo {
                    content_type: repr.content_type.clone(),
                    schema,
                }
            })
            .collect(),
        raises: op
            .meta
            .raises
            .iter()
            .filter_map(|refer| {
                let exception = exceptions
                    .iter()
                    .find(|e| e.ident == refer.ident && e.module_path == refer.module_path)?;
                let mut headers: Vec<_> = exception
                    .headers
                    .iter()
                    .map(|member| (member.wire_name.clone(), schema_for_type(&member.ty)))
                    .collect();
                if !exception.cookies.is_empty() {
                    headers.push((
                        "Set-Cookie".into(),
                        ObjectBuilder::new().schema_type(Type::String).into(),
                    ));
                }
                let schema = (!exception.body.is_empty()).then(|| {
                    schema_ref(&super::naming::scoped_name(
                        &exception.module_path,
                        &exception.ident,
                    ))
                });
                Some(ExceptionResponseInfo {
                    status: exception.status.to_string(),
                    ident: exception.ident.clone(),
                    headers,
                    schema,
                })
            })
            .collect(),
    }
}

fn validate_stream_contract(op: &HttpOperation) {
    match op.meta.stream.kind {
        Some(HttpStreamKind::Server) if op.meta.stream.codec != HttpStreamCodec::Sse => {
            panic!(
                "openapi currently supports only SSE for @server_stream methods: '{}'",
                op.meta.name
            );
        }
        Some(HttpStreamKind::Client) if op.meta.stream.codec != HttpStreamCodec::Ndjson => {
            panic!(
                "openapi currently supports only NDJSON for @client_stream methods: '{}'",
                op.meta.name
            );
        }
        _ => {}
    }
}
