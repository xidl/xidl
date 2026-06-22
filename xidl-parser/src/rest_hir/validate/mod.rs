use crate::hir;
use std::collections::{HashMap, HashSet};

use super::semantics::{
    DeprecatedInfo, HttpStreamCodec, HttpStreamKind, annotation_name, annotation_params,
    deprecated_info, normalize_annotation_params,
};
use super::{HttpMethod, HttpParam, HttpParamKind, HttpRoute, RestHirResult};

#[cfg(test)]
mod tests;

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

pub(super) fn validate_header_name(bound_name: &str, param_name: &str) -> RestHirResult<()> {
    if bound_name.is_empty() {
        return Err(format!("parameter '{}' has empty @header name", param_name));
    }
    if bound_name.starts_with(':') {
        return Err(format!(
            "parameter '{}' uses reserved pseudo-header name '{}'",
            param_name, bound_name
        ));
    }
    Ok(())
}

pub(super) fn validate_cookie_name(bound_name: &str, param_name: &str) -> RestHirResult<()> {
    if bound_name.is_empty() {
        return Err(format!("parameter '{}' has empty @cookie name", param_name));
    }
    if bound_name
        .chars()
        .any(|ch| ch.is_ascii_whitespace() || ch == ';' || ch == '=')
    {
        return Err(format!(
            "parameter '{}' has invalid @cookie name '{}'",
            param_name, bound_name
        ));
    }
    Ok(())
}

pub(super) fn validate_stream_shape(
    op_name: &str,
    stream: super::semantics::HttpStreamConfig,
) -> RestHirResult<()> {
    if matches!(
        stream.kind,
        Some(HttpStreamKind::Client | HttpStreamKind::Bidi)
    ) && stream.codec == HttpStreamCodec::Sse
    {
        return Err(format!(
            "@stream_codec(\"sse\") requires @server_stream on method '{}'",
            op_name
        ));
    }
    Ok(())
}

pub(super) fn validate_stream_method(
    op_name: &str,
    stream_kind: Option<HttpStreamKind>,
    method: HttpMethod,
) -> RestHirResult<()> {
    let expected = match stream_kind {
        Some(HttpStreamKind::Server) => Some((method, HttpMethod::Get, "@server_stream")),
        Some(HttpStreamKind::Client) => Some((method, HttpMethod::Post, "@client_stream")),
        Some(HttpStreamKind::Bidi) => Some((method, HttpMethod::Get, "@bidi_stream")),
        None => None,
    };
    if let Some((actual, required, label)) = expected
        && actual != required
    {
        return Err(format!(
            "{label} method '{}' must use {}",
            op_name,
            http_method_name(required)
        ));
    }
    Ok(())
}

pub(super) fn validate_projected_param(
    op_name: &str,
    param: &HttpParam,
    direction: HttpParamDirection,
    route_path_names: &[HashSet<String>],
) -> RestHirResult<()> {
    if matches!(direction, HttpParamDirection::Out) && param.flatten {
        return Err(format!(
            "@flatten can only be applied to request-side body parameter '{}' of method '{}'",
            param.name, op_name
        ));
    }
    if !matches!(param.kind, HttpParamKind::Path) {
        return Ok(());
    }
    if route_path_names
        .iter()
        .any(|set| set.contains(&param.wire_name))
    {
        if param.optional {
            return Err(format!(
                "@optional cannot be applied to path parameter '{}' of method '{}'",
                param.name, op_name
            ));
        }
        if !route_path_names
            .iter()
            .all(|set| set.contains(&param.wire_name))
        {
            return Err(format!(
                "parameter '{}' is bound to path variable '{}' but it is not present in every route template of method '{}'",
                param.name, param.wire_name, op_name
            ));
        }
        return Ok(());
    }
    Err(format!(
        "parameter '{}' is annotated with @path but '{}' is not present in any route template of method '{}'",
        param.name, param.wire_name, op_name
    ))
}

pub(super) fn validate_route_bindings(
    op_name: &str,
    routes: &[HttpRoute],
    path_binding_count: &HashMap<String, usize>,
    query_binding_count: &HashMap<String, usize>,
) -> RestHirResult<()> {
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
) -> RestHirResult<()> {
    for route_param in route_params {
        match binding_count.get(route_param).copied().unwrap_or(0) {
            0 => {
                return Err(format!(
                    "{label} '{}' has no matching {binding_label} in method '{}'",
                    route_param, op_name
                ));
            }
            1 => {}
            _ => {
                return Err(format!(
                    "{label} '{}' is bound by multiple {}s in method '{}'",
                    route_param, binding_label, op_name
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn validate_request_shape(
    op_name: &str,
    stream_kind: Option<HttpStreamKind>,
    request_params: &[HttpParam],
) -> RestHirResult<()> {
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
        return Err(format!(
            "{label} method '{}' currently supports body parameters only",
            op_name
        ));
    }
    if request_body_params.len() != 1 && request_body_params.iter().any(|param| param.flatten) {
        return Err(format!(
            "@flatten requires exactly one request-side body parameter, but method '{}' has {}",
            op_name,
            request_body_params.len()
        ));
    }
    Ok(())
}

pub(super) fn validate_head_constraints(
    op_name: &str,
    method: HttpMethod,
    response_params: &[HttpParam],
    return_type: Option<&hir::TypeSpec>,
) -> RestHirResult<()> {
    if matches!(method, HttpMethod::Head) && (return_type.is_some() || !response_params.is_empty())
    {
        return Err(format!("HEAD method '{}' must return void", op_name));
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

pub(super) fn validate_upgrade_constraints(
    op_name: &str,
    has_upgrade: bool,
    upgrade_protocol: Option<&str>,
    method: HttpMethod,
    request_params: &[HttpParam],
    return_type: Option<&hir::TypeSpec>,
) -> RestHirResult<()> {
    if !has_upgrade {
        return Ok(());
    }
    let Some(proto) = upgrade_protocol else {
        return Err(format!(
            "@upgrade annotation on method '{}' requires a 'protocol' parameter (e.g. @upgrade(protocol=\"xidl-raw\"))",
            op_name
        ));
    };
    if proto.is_empty() {
        return Err(format!(
            "@upgrade 'protocol' parameter on method '{}' cannot be empty",
            op_name
        ));
    }
    if return_type.is_some() {
        return Err(format!("@upgrade method '{}' must return void", op_name));
    }
    let has_body_param = request_params
        .iter()
        .any(|param| matches!(param.kind, HttpParamKind::Body));
    if has_body_param {
        return Err(format!(
            "@upgrade method '{}' cannot have @body parameters",
            op_name
        ));
    }
    if method != HttpMethod::Get {
        return Err(format!("@upgrade method '{}' must use GET", op_name));
    }
    Ok(())
}
