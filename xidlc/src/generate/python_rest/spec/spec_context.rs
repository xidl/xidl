use crate::error::IdlcResult;
use xidl_parser::rest_hir::{
    HttpOperation, HttpParam,
    semantics::{HttpStreamCodec, HttpStreamKind},
};

use super::spec_meta::{
    http_method_name, param_source, security_expr, stream_expr, validate_stream_support,
};
use super::spec_types::{py_field_name, py_type, py_type_name};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ParamSource {
    Path,
    Query,
    Header,
    Cookie,
    Body,
}

#[derive(Clone)]
pub(super) struct ParamContext {
    pub(super) field_name: String,
    pub(super) wire_name: String,
    pub(super) ty: String,
    pub(super) optional: bool,
    pub(super) source: ParamSource,
    pub(super) flatten: bool,
}

pub(super) struct MethodContext {
    pub(super) method_name: String,
    pub(super) raw_name: String,
    pub(super) endpoint_name: String,
    pub(super) route_builder_name: String,
    pub(super) http_method: String,
    pub(super) paths: Vec<String>,
    pub(super) request_type: String,
    pub(super) response_type: String,
    pub(super) request_content_type: String,
    pub(super) response_content_type: String,
    pub(super) requires_request_content_type: bool,
    pub(super) security_expr: String,
    pub(super) stream_expr: String,
    pub(super) stream_kind: Option<HttpStreamKind>,
    pub(super) stream_codec: HttpStreamCodec,
    pub(super) response_body_shape: xidl_parser::rest_hir::HttpBodyShape,
    pub(super) request_params: Vec<ParamContext>,
    pub(super) response_params: Vec<ParamContext>,
    pub(super) return_ty: Option<String>,
}

pub(super) fn build_method(
    operation: &HttpOperation,
    interface_name: &str,
) -> IdlcResult<MethodContext> {
    validate_stream_support(operation)?;

    let stream_kind = operation.stream.kind;
    let stream_codec = operation.stream.codec;
    let request_params = operation.request_params.iter().map(param_context).collect();
    let response_params = operation
        .response_params
        .iter()
        .map(param_context)
        .collect();

    Ok(MethodContext {
        method_name: py_field_name(&operation.name),
        raw_name: operation.name.clone(),
        endpoint_name: format!(
            "_{}_{}_endpoint",
            py_field_name(interface_name),
            py_field_name(&operation.name)
        ),
        route_builder_name: format!(
            "_{}_{}_route",
            py_field_name(interface_name),
            py_field_name(&operation.name)
        ),
        http_method: http_method_name(operation.method).to_string(),
        paths: operation
            .routes
            .iter()
            .map(|route| route.path.clone())
            .collect(),
        request_type: format!("{}{}Request", interface_name, py_type_name(&operation.name)),
        response_type: response_type(interface_name, operation.name.as_str(), stream_kind),
        request_content_type: operation.request_content_type.clone(),
        response_content_type: operation.response_content_type.clone(),
        requires_request_content_type: !matches!(
            operation.request_body_shape,
            xidl_parser::rest_hir::HttpBodyShape::None
        ),
        security_expr: security_expr(operation.security.as_ref()),
        stream_expr: stream_expr(operation.stream),
        stream_kind,
        stream_codec,
        response_body_shape: operation.response_body_shape,
        request_params,
        response_params,
        return_ty: operation.return_type.as_ref().map(py_type),
    })
}

fn response_type(
    interface_name: &str,
    operation_name: &str,
    stream_kind: Option<HttpStreamKind>,
) -> String {
    match stream_kind {
        Some(HttpStreamKind::Server) => "ServerStreamResponse".to_string(),
        Some(HttpStreamKind::Client) | None => {
            format!("{}{}Response", interface_name, py_type_name(operation_name))
        }
        Some(HttpStreamKind::Bidi) => unreachable!(),
    }
}

fn param_context(param: &HttpParam) -> ParamContext {
    ParamContext {
        field_name: py_field_name(&param.name),
        wire_name: param.wire_name.clone(),
        ty: py_type(&param.ty),
        optional: param.optional,
        source: param_source(param.kind),
        flatten: param.flatten,
    }
}
