use crate::error::IdlcResult;
use std::collections::{HashMap, HashSet};
use xidl_parser::hir;

use super::route::{SourceBinding, explicit_param_binding};
use super::validate::{
    default_param_source, param_direction, validate_cookie_name, validate_header_name,
    validate_projected_param,
};
use super::semantics::{HttpStreamKind, has_annotation, has_optional_annotation};
use super::{HttpParam, HttpParamDirection, HttpParamSource};

#[allow(clippy::type_complexity)]
pub(super) fn project_params(
    op: &hir::OpDcl,
    method: super::HttpMethod,
    stream_kind: Option<HttpStreamKind>,
    route_path_names: &[HashSet<String>],
    route_query_names: &[HashSet<String>],
) -> IdlcResult<(
    Vec<HttpParam>,
    Vec<HttpParam>,
    Vec<HttpParam>,
    Vec<HttpParam>,
    Vec<HttpParam>,
    Vec<HttpParam>,
    Vec<HttpParam>,
    Vec<HttpParam>,
    Vec<HttpParam>,
    Vec<HttpParam>,
    HashMap<String, usize>,
    HashMap<String, usize>,
)> {
    let params = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);
    let mut request_params = Vec::new();
    let mut request_path_params = Vec::new();
    let mut request_query_params = Vec::new();
    let mut request_header_params = Vec::new();
    let mut request_cookie_params = Vec::new();
    let mut request_body_params = Vec::new();
    let mut response_params = Vec::new();
    let mut response_header_params = Vec::new();
    let mut response_cookie_params = Vec::new();
    let mut response_body_params = Vec::new();
    let mut path_binding_count = HashMap::<String, usize>::new();
    let mut query_binding_count = HashMap::<String, usize>::new();

    for param in params {
        let direction = param_direction(param.attr.as_ref());
        let binding = explicit_param_binding(param)?;
        let default_source = if matches!(stream_kind, Some(HttpStreamKind::Bidi)) {
            HttpParamSource::Body
        } else {
            default_param_source(method)
        };
        let inferred_source = infer_param_source(
            param,
            direction,
            binding.as_ref(),
            default_source,
            route_path_names,
            route_query_names,
        );
        let projected = HttpParam {
            name: param.declarator.0.clone(),
            wire_name: binding
                .as_ref()
                .map(|value| value.bound_name.clone())
                .unwrap_or_else(|| param.declarator.0.clone()),
            ty: param.ty.clone(),
            direction,
            source: inferred_source,
            optional: has_optional_annotation(&param.annotations),
            flatten: has_annotation(&param.annotations, "flatten"),
        };
        if matches!(projected.source, HttpParamSource::Header) {
            validate_header_name(&projected.wire_name, &projected.name)?;
        }
        if matches!(projected.source, HttpParamSource::Cookie) {
            validate_cookie_name(&projected.wire_name, &projected.name)?;
        }
        validate_projected_param(&op.ident, &projected, direction, route_path_names)?;
        distribute_param(
            projected,
            direction,
            &mut request_params,
            &mut request_path_params,
            &mut request_query_params,
            &mut request_header_params,
            &mut request_cookie_params,
            &mut request_body_params,
            &mut response_params,
            &mut response_header_params,
            &mut response_cookie_params,
            &mut response_body_params,
            &mut path_binding_count,
            &mut query_binding_count,
        );
    }

    Ok((
        request_params,
        request_path_params,
        request_query_params,
        request_header_params,
        request_cookie_params,
        request_body_params,
        response_params,
        response_header_params,
        response_cookie_params,
        response_body_params,
        path_binding_count,
        query_binding_count,
    ))
}

fn infer_param_source(
    param: &hir::ParamDcl,
    direction: HttpParamDirection,
    binding: Option<&SourceBinding>,
    default_source: HttpParamSource,
    route_path_names: &[HashSet<String>],
    route_query_names: &[HashSet<String>],
) -> HttpParamSource {
    match direction {
        HttpParamDirection::Out | HttpParamDirection::InOut => binding
            .map(|value| value.source)
            .unwrap_or(HttpParamSource::Body),
        HttpParamDirection::In => binding.map(|value| value.source).unwrap_or_else(|| {
            if route_path_names
                .iter()
                .all(|set| set.contains(&param.declarator.0))
            {
                HttpParamSource::Path
            } else if route_query_names
                .iter()
                .any(|set| set.contains(&param.declarator.0))
            {
                HttpParamSource::Query
            } else {
                default_source
            }
        }),
    }
}

#[allow(clippy::too_many_arguments)]
fn distribute_param(
    projected: HttpParam,
    direction: HttpParamDirection,
    request_params: &mut Vec<HttpParam>,
    request_path_params: &mut Vec<HttpParam>,
    request_query_params: &mut Vec<HttpParam>,
    request_header_params: &mut Vec<HttpParam>,
    request_cookie_params: &mut Vec<HttpParam>,
    request_body_params: &mut Vec<HttpParam>,
    response_params: &mut Vec<HttpParam>,
    response_header_params: &mut Vec<HttpParam>,
    response_cookie_params: &mut Vec<HttpParam>,
    response_body_params: &mut Vec<HttpParam>,
    path_binding_count: &mut HashMap<String, usize>,
    query_binding_count: &mut HashMap<String, usize>,
) {
    let projected_source = projected.source;
    if matches!(
        direction,
        HttpParamDirection::In | HttpParamDirection::InOut
    ) {
        request_params.push(projected.clone());
        match projected_source {
            HttpParamSource::Path => {
                *path_binding_count
                    .entry(projected.wire_name.clone())
                    .or_insert(0) += 1;
                request_path_params.push(projected.clone());
            }
            HttpParamSource::Query => {
                *query_binding_count
                    .entry(projected.wire_name.clone())
                    .or_insert(0) += 1;
                request_query_params.push(projected.clone());
            }
            HttpParamSource::Header => request_header_params.push(projected.clone()),
            HttpParamSource::Cookie => request_cookie_params.push(projected.clone()),
            HttpParamSource::Body => request_body_params.push(projected.clone()),
        }
    }
    if matches!(
        direction,
        HttpParamDirection::Out | HttpParamDirection::InOut
    ) {
        response_params.push(projected.clone());
        match projected_source {
            HttpParamSource::Header => response_header_params.push(projected),
            HttpParamSource::Cookie => response_cookie_params.push(projected),
            _ => response_body_params.push(projected),
        }
    }
}
