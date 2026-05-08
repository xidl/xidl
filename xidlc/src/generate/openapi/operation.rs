use super::naming::{method_to_openapi, openapi_path_template, operation_id};
use super::schema::{
    apply_deprecation_note, array_schema, body_payload_schema, parameter_schema,
    request_body_schema, schema_for_type,
};
use super::security::openapi_security_requirement;
use crate::openapi::path::{HttpMethod as OpenApiHttpMethod, Parameter, ParameterIn};
use crate::openapi::request_body::RequestBody;
use crate::openapi::schema::Schema;
use crate::openapi::{RefOr, security::SecurityRequirement};
use xidl_parser::rest_hir::{
    HttpMethod as RestHirMethod, HttpOperation, HttpParamKind as RestHirParamKind,
    semantics::{HttpSecurityRequirement, HttpStreamCodec, HttpStreamKind},
};

pub(crate) struct MethodInfo {
    pub(crate) http_method: OpenApiHttpMethod,
    pub(crate) paths: Vec<String>,
    pub(crate) operation_id: String,
    pub(crate) parameters: Vec<Parameter>,
    pub(crate) request_body: Option<RequestBody>,
    pub(crate) request_stream_item_schema: Option<RefOr<Schema>>,
    pub(crate) response_status: &'static str,
    pub(crate) response_schema: Option<RefOr<Schema>>,
    pub(crate) response_stream_item_schema: Option<RefOr<Schema>>,
    pub(crate) summary: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) deprecated: bool,
    pub(crate) security_requirements: Option<Vec<HttpSecurityRequirement>>,
    pub(crate) security: Option<Vec<SecurityRequirement>>,
    pub(crate) response_content_type: String,
}

pub(crate) fn render_http_operation(
    op: &HttpOperation,
    module_path: &[String],
    interface_name: &str,
) -> MethodInfo {
    validate_stream_contract(op);

    let mut parameters = Vec::new();
    let mut body_props = Vec::new();
    let body_required = Vec::new();
    let mut output_fields = Vec::new();

    for param in &op.request_params {
        let schema = schema_for_type(&param.ty);
        match param.kind {
            RestHirParamKind::Path => parameters.push(parameter_schema(
                ParameterIn::Path,
                &param.wire_name,
                schema,
                true,
                None,
            )),
            RestHirParamKind::Query => parameters.push(parameter_schema(
                ParameterIn::Query,
                &param.wire_name,
                schema,
                false,
                None,
            )),
            RestHirParamKind::Header => parameters.push(parameter_schema(
                ParameterIn::Header,
                &param.wire_name,
                schema,
                false,
                None,
            )),
            RestHirParamKind::Cookie => parameters.push(parameter_schema(
                ParameterIn::Cookie,
                &param.wire_name,
                schema,
                false,
                None,
            )),
            RestHirParamKind::Body => body_props.push((param.name.clone(), schema)),
        }
    }

    for param in &op.response_params {
        output_fields.push((param.wire_name.clone(), schema_for_type(&param.ty)));
    }

    let return_schema = op.return_type.as_ref().map(schema_for_type);
    let (response_status, mut response_schema) = response_shape(
        return_schema,
        output_fields,
        matches!(op.method, RestHirMethod::Head),
    );
    let mut request_schema = body_payload_schema(body_props, body_required);
    if matches!(op.stream.kind, Some(HttpStreamKind::Bidi)) {
        request_schema = request_schema.map(array_schema);
        response_schema = response_schema.map(array_schema);
    }

    let (security_requirements, security) = op
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
            .clone()
            .map(|schema| request_body_schema(schema, &op.request_content_type)),
        request_stream_item_schema: matches!(op.stream.kind, Some(HttpStreamKind::Client))
            .then_some(request_schema)
            .flatten(),
        response_status,
        response_schema: response_schema.clone(),
        response_stream_item_schema: matches!(op.stream.kind, Some(HttpStreamKind::Server))
            .then_some(response_schema)
            .flatten(),
        summary: None,
        description: apply_deprecation_note(None, op.deprecated.as_ref()),
        deprecated: op
            .deprecated
            .as_ref()
            .map(|value| value.deprecated)
            .unwrap_or(false),
        security_requirements,
        security,
        response_content_type: op.response_content_type.clone(),
    }
}

fn validate_stream_contract(op: &HttpOperation) {
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

fn response_shape(
    return_schema: Option<RefOr<Schema>>,
    output_fields: Vec<(String, RefOr<Schema>)>,
    is_head: bool,
) -> (&'static str, Option<RefOr<Schema>>) {
    let output_count = usize::from(return_schema.is_some()) + output_fields.len();
    if is_head || output_count == 0 {
        return ("204", None);
    }
    if output_count == 1 {
        if let Some(schema) = return_schema {
            return ("200", Some(schema));
        }
        if let Some((_, schema)) = output_fields.first().cloned() {
            return ("200", Some(schema));
        }
    }

    let mut object = crate::openapi::schema::ObjectBuilder::new()
        .schema_type(crate::openapi::schema::Type::Object);
    if let Some(schema) = return_schema {
        object = object.property("return", schema).required("return");
    }
    for (name, schema) in output_fields {
        object = object
            .property(name.as_str(), schema)
            .required(name.as_str());
    }
    ("200", Some(RefOr::T(Schema::from(object))))
}
