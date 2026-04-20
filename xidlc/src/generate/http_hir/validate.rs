use crate::error::{IdlcError, IdlcResult};
use std::collections::{HashMap, HashSet};
use xidl_parser::hir;

use super::semantics::{
    DeprecatedInfo, HttpStreamCodec, HttpStreamKind, annotation_name, annotation_params,
    deprecated_info, normalize_annotation_params,
};
use super::{HttpMethod, HttpParam, HttpParamKind, HttpRoute};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HttpParamDirection {
    In,
    Out,
    InOut,
}

pub(super) fn default_param_source(method: HttpMethod) -> HttpParamKind {
    match method {
        HttpMethod::Get | HttpMethod::Delete | HttpMethod::Head | HttpMethod::Options => {
            HttpParamKind::Query
        }
        HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch => HttpParamKind::Body,
    }
}

pub(super) fn param_direction(attr: Option<&hir::ParamAttribute>) -> HttpParamDirection {
    match attr.map(|value| value.0.as_str()) {
        Some("out") => HttpParamDirection::Out,
        Some("inout") => HttpParamDirection::InOut,
        _ => HttpParamDirection::In,
    }
}

pub(super) fn effective_deprecated(
    interface_annotations: &[hir::Annotation],
    item_annotations: &[hir::Annotation],
) -> Result<Option<DeprecatedInfo>, String> {
    deprecated_info(item_annotations).and_then(|value| {
        if value.is_some() {
            Ok(value)
        } else {
            deprecated_info(interface_annotations)
        }
    })
}

pub(super) fn effective_basic_auth_realm(
    interface_annotations: &[hir::Annotation],
    item_annotations: &[hir::Annotation],
) -> Option<String> {
    find_basic_auth_realm(item_annotations).or_else(|| find_basic_auth_realm(interface_annotations))
}

fn find_basic_auth_realm(annotations: &[hir::Annotation]) -> Option<String> {
    annotations.iter().find_map(|annotation| {
        let name = annotation_name(annotation)?;
        if !name.eq_ignore_ascii_case("http_basic") {
            return None;
        }
        annotation_params(annotation)
            .map(normalize_annotation_params)
            .and_then(|params| params.get("realm").cloned())
            .filter(|value| !value.is_empty())
    })
}

pub(super) fn validate_header_name(bound_name: &str, param_name: &str) -> IdlcResult<()> {
    if bound_name.is_empty() {
        return Err(IdlcError::rpc(format!(
            "parameter '{}' has empty @header name",
            param_name
        )));
    }
    if bound_name.starts_with(':') {
        return Err(IdlcError::rpc(format!(
            "parameter '{}' uses reserved pseudo-header name '{}'",
            param_name, bound_name
        )));
    }
    Ok(())
}

pub(super) fn validate_cookie_name(bound_name: &str, param_name: &str) -> IdlcResult<()> {
    if bound_name.is_empty() {
        return Err(IdlcError::rpc(format!(
            "parameter '{}' has empty @cookie name",
            param_name
        )));
    }
    if bound_name
        .chars()
        .any(|ch| ch.is_ascii_whitespace() || ch == ';' || ch == '=')
    {
        return Err(IdlcError::rpc(format!(
            "parameter '{}' has invalid @cookie name '{}'",
            param_name, bound_name
        )));
    }
    Ok(())
}

pub(super) fn validate_stream_shape(
    op_name: &str,
    stream: super::semantics::HttpStreamConfig,
) -> IdlcResult<()> {
    if matches!(
        stream.kind,
        Some(HttpStreamKind::Client | HttpStreamKind::Bidi)
    ) && stream.codec == HttpStreamCodec::Sse
    {
        return Err(IdlcError::rpc(format!(
            "@stream_codec(\"sse\") requires @server_stream on method '{}'",
            op_name
        )));
    }
    Ok(())
}

pub(super) fn validate_stream_method(
    op_name: &str,
    stream_kind: Option<HttpStreamKind>,
    method: HttpMethod,
) -> IdlcResult<()> {
    let expected = match stream_kind {
        Some(HttpStreamKind::Server) => Some((method, HttpMethod::Get, "@server_stream")),
        Some(HttpStreamKind::Client) => Some((method, HttpMethod::Post, "@client_stream")),
        Some(HttpStreamKind::Bidi) => Some((method, HttpMethod::Get, "@bidi_stream")),
        None => None,
    };
    if let Some((actual, required, label)) = expected
        && actual != required
    {
        return Err(IdlcError::rpc(format!(
            "{label} method '{}' must use {}",
            op_name,
            http_method_name(required)
        )));
    }
    Ok(())
}

pub(super) fn validate_projected_param(
    op_name: &str,
    param: &HttpParam,
    direction: HttpParamDirection,
    route_path_names: &[HashSet<String>],
) -> IdlcResult<()> {
    if matches!(direction, HttpParamDirection::Out) && param.flatten {
        return Err(IdlcError::rpc(format!(
            "@flatten can only be applied to request-side body parameter '{}' of method '{}'",
            param.name, op_name
        )));
    }
    if matches!(param.kind, HttpParamKind::Path) {
        if route_path_names
            .iter()
            .any(|set| set.contains(&param.wire_name))
        {
            if param.optional {
                return Err(IdlcError::rpc(format!(
                    "@optional cannot be applied to path parameter '{}' of method '{}'",
                    param.name, op_name
                )));
            }
            if !route_path_names
                .iter()
                .all(|set| set.contains(&param.wire_name))
            {
                return Err(IdlcError::rpc(format!(
                    "parameter '{}' is bound to path variable '{}' but it is not present in every route template of method '{}'",
                    param.name, param.wire_name, op_name
                )));
            }
        } else {
            return Err(IdlcError::rpc(format!(
                "parameter '{}' is annotated with @path but '{}' is not present in any route template of method '{}'",
                param.name, param.wire_name, op_name
            )));
        }
    }
    Ok(())
}

pub(super) fn validate_route_bindings(
    op_name: &str,
    routes: &[HttpRoute],
    path_binding_count: &HashMap<String, usize>,
    query_binding_count: &HashMap<String, usize>,
) -> IdlcResult<()> {
    for route in routes {
        validate_route_group(
            op_name,
            &route.path_params,
            path_binding_count,
            "route template variable",
            "request-side path parameter",
        )?;
        validate_route_group(
            op_name,
            &route.query_params,
            query_binding_count,
            "query template variable",
            "request-side query parameter",
        )?;
    }
    Ok(())
}

fn validate_route_group(
    op_name: &str,
    route_params: &[String],
    binding_count: &HashMap<String, usize>,
    label: &str,
    binding_label: &str,
) -> IdlcResult<()> {
    for route_param in route_params {
        match binding_count.get(route_param).copied().unwrap_or(0) {
            0 => {
                return Err(IdlcError::rpc(format!(
                    "{label} '{}' has no matching {binding_label} in method '{}'",
                    route_param, op_name
                )));
            }
            1 => {}
            _ => {
                return Err(IdlcError::rpc(format!(
                    "{label} '{}' is bound by multiple {}s in method '{}'",
                    route_param, binding_label, op_name
                )));
            }
        }
    }
    Ok(())
}

pub(super) fn validate_request_shape(
    op_name: &str,
    stream_kind: Option<HttpStreamKind>,
    request_params: &[HttpParam],
) -> IdlcResult<()> {
    let request_body_params = request_params
        .iter()
        .filter(|param| matches!(param.kind, HttpParamKind::Body))
        .collect::<Vec<_>>();
    let has_non_body_request_params = request_params
        .iter()
        .any(|param| !matches!(param.kind, HttpParamKind::Body));
    if matches!(
        stream_kind,
        Some(HttpStreamKind::Client | HttpStreamKind::Bidi)
    ) && has_non_body_request_params
    {
        let label = if matches!(stream_kind, Some(HttpStreamKind::Bidi)) {
            "@bidi_stream"
        } else {
            "@client_stream"
        };
        return Err(IdlcError::rpc(format!(
            "{label} method '{}' currently supports body parameters only",
            op_name
        )));
    }
    if request_body_params.len() != 1 && request_body_params.iter().any(|param| param.flatten) {
        return Err(IdlcError::rpc(format!(
            "@flatten requires exactly one request-side body parameter, but method '{}' has {}",
            op_name,
            request_body_params.len()
        )));
    }
    Ok(())
}

pub(super) fn validate_head_constraints(
    op_name: &str,
    method: HttpMethod,
    response_params: &[HttpParam],
    return_type: Option<&hir::TypeSpec>,
) -> IdlcResult<()> {
    if matches!(method, HttpMethod::Head) && (return_type.is_some() || !response_params.is_empty())
    {
        return Err(IdlcError::rpc(format!(
            "HEAD method '{}' must return void",
            op_name
        )));
    }
    Ok(())
}

pub(super) fn http_method_name(method: HttpMethod) -> &'static str {
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
