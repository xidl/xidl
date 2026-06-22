use crate::error::IdlcResult;
use xidl_parser::rest_hir::{
    HttpOperation, HttpRequestBodyShape,
    semantics::{HttpStreamCodec, HttpStreamKind},
};

use super::spec_meta::{http_method_name, security_expr, stream_expr, validate_stream_support};
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
    pub(super) response_content_type_explicit: bool,
    pub(super) requires_request_content_type: bool,
    pub(super) security_expr: String,
    pub(super) stream_expr: String,
    pub(super) stream_kind: Option<HttpStreamKind>,
    pub(super) stream_codec: HttpStreamCodec,
    pub(super) request_params: Vec<ParamContext>,
    pub(super) response_params: Vec<ParamContext>,
    pub(super) return_ty: Option<String>,
}

pub(super) fn build_method(
    operation: &HttpOperation,
    interface_name: &str,
) -> IdlcResult<MethodContext> {
    validate_stream_support(operation)?;

    let stream_kind = operation.meta.stream.kind;
    let stream_codec = operation.meta.stream.codec;

    let request_params = operation
        .signature
        .params
        .iter()
        .filter(|p| {
            matches!(
                p.direction,
                xidl_parser::rest_hir::HttpSignatureParamDirection::In
                    | xidl_parser::rest_hir::HttpSignatureParamDirection::InOut
            )
        })
        .map(|p| {
            let optional = p.is_optional;

            // Find source and wire name from op.http
            let (source, wire_name, flatten) = find_input_binding(operation, &p.name);

            ParamContext {
                field_name: py_field_name(&p.name),
                wire_name,
                ty: py_type(&p.ty),
                optional,
                source,
                flatten,
            }
        })
        .collect::<Vec<_>>();

    let response_params = operation
        .signature
        .params
        .iter()
        .filter(|p| {
            matches!(
                p.direction,
                xidl_parser::rest_hir::HttpSignatureParamDirection::Out
                    | xidl_parser::rest_hir::HttpSignatureParamDirection::InOut
            )
        })
        .map(|p| {
            let optional = p.annotations.iter().any(|a| {
                matches!(
                    a,
                    xidl_parser::rest_hir::HttpSignatureParamAnnotation::Optional
                )
            });

            // For response, we currently only care about Body params for the Response class
            // Headers/Cookies are handled differently in python-rest?
            // Actually python-rest seems to put EVERYTHING in the Response class
            let (source, wire_name) = find_output_binding(operation, &p.name);

            ParamContext {
                field_name: py_field_name(&p.name),
                wire_name,
                ty: py_type(&p.ty),
                optional,
                source,
                flatten: false,
            }
        })
        .collect::<Vec<_>>();

    Ok(MethodContext {
        method_name: py_field_name(&operation.meta.name),
        raw_name: operation.meta.name.clone(),
        endpoint_name: format!(
            "_{}_{}_endpoint",
            py_field_name(interface_name),
            py_field_name(&operation.meta.name)
        ),
        route_builder_name: format!(
            "_{}_{}_route",
            py_field_name(interface_name),
            py_field_name(&operation.meta.name)
        ),
        http_method: http_method_name(operation.meta.method).to_string(),
        paths: operation
            .meta
            .routes
            .iter()
            .map(|route| route.path.clone())
            .collect(),
        request_type: format!(
            "{}{}Request",
            interface_name,
            py_type_name(&operation.meta.name)
        ),
        response_type: response_type(interface_name, operation.meta.name.as_str(), stream_kind),
        request_content_type: operation
            .http
            .request
            .body
            .content_type
            .clone()
            .unwrap_or_default(),
        response_content_type: operation
            .http
            .response
            .body
            .content_type
            .clone()
            .unwrap_or_default(),
        response_content_type_explicit: operation.http.response.body.content_type_explicit,
        requires_request_content_type: !matches!(
            operation.http.request.body.shape,
            HttpRequestBodyShape::Empty
        ),
        security_expr: security_expr(
            operation.meta.security.as_ref(),
            operation.meta.basic_auth_realm.as_ref(),
        ),
        stream_expr: stream_expr(operation.meta.stream),
        stream_kind,
        stream_codec,
        request_params,
        response_params,
        return_ty: operation.signature.return_type.as_ref().map(py_type),
    })
}

fn find_input_binding(op: &HttpOperation, name: &str) -> (ParamSource, String, bool) {
    if let Some(b) = op.http.request.path.iter().find(|b| b.source_param == name) {
        return (ParamSource::Path, b.wire_name.clone(), false);
    }
    if let Some(b) = op
        .http
        .request
        .query
        .iter()
        .find(|b| b.source_param == name)
    {
        return (ParamSource::Query, b.wire_name.clone(), false);
    }
    if let Some(b) = op
        .http
        .request
        .header
        .iter()
        .find(|b| b.source_param == name)
    {
        return (ParamSource::Header, b.wire_name.clone(), false);
    }
    if let Some(b) = op
        .http
        .request
        .cookie
        .iter()
        .find(|b| b.source_param == name)
    {
        return (ParamSource::Cookie, b.wire_name.clone(), false);
    }

    match &op.http.request.body.shape {
        HttpRequestBodyShape::SingleValue {
            source_param,
            flatten,
            ..
        } if source_param == name => {
            let is_text = matches!(
                op.http.request.body.codec,
                Some(xidl_parser::rest_hir::HttpBodyCodec::Text)
            );
            (
                ParamSource::Body,
                if *flatten || is_text {
                    "".to_string()
                } else {
                    name.to_string()
                },
                *flatten || is_text,
            )
        }
        HttpRequestBodyShape::Object { fields } => {
            if let Some(f) = fields.iter().find(|f| f.source_param == name) {
                (ParamSource::Body, f.field_name.clone(), f.flatten)
            } else {
                (ParamSource::Body, name.to_string(), false)
            }
        }
        HttpRequestBodyShape::Stream { source_param, .. } if source_param == name => {
            (ParamSource::Body, name.to_string(), false)
        }
        _ => (ParamSource::Body, name.to_string(), false),
    }
}

fn find_output_binding(op: &HttpOperation, name: &str) -> (ParamSource, String) {
    if let Some(b) = op.http.response.header.iter().find(|b| match &b.source {
        xidl_parser::rest_hir::HttpOutputSource::Param { name: n } => n == name,
        _ => false,
    }) {
        return (ParamSource::Header, b.wire_name.clone());
    }
    if let Some(b) = op.http.response.cookie.iter().find(|b| match &b.source {
        xidl_parser::rest_hir::HttpOutputSource::Param { name: n } => n == name,
        _ => false,
    }) {
        return (ParamSource::Cookie, b.wire_name.clone());
    }
    (ParamSource::Body, name.to_string())
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
