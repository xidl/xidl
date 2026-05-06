use crate::hir;
use std::collections::{HashMap, HashSet};

use super::route::{SourceBinding, explicit_param_binding};
use super::semantics::{HttpStreamKind, has_annotation, has_optional_annotation};
use super::validate::{
    HttpParamDirection, default_param_source, param_direction, validate_cookie_name,
    validate_header_name, validate_projected_param,
};
use super::{HttpParam, HttpParamKind, RestHirResult};

#[cfg(test)]
mod tests;

#[allow(clippy::type_complexity)]
pub(super) fn project_params(
    op: &hir::OpDcl,
    method: super::HttpMethod,
    stream_kind: Option<HttpStreamKind>,
    route_path_names: &[HashSet<String>],
    route_query_names: &[HashSet<String>],
) -> RestHirResult<(
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
    let mut response_params = Vec::new();
    let mut path_binding_count = HashMap::<String, usize>::new();
    let mut query_binding_count = HashMap::<String, usize>::new();

    for param in params {
        let direction = param_direction(param.attr.as_ref());
        let binding = explicit_param_binding(param)?;
        let default_source = if matches!(stream_kind, Some(HttpStreamKind::Bidi)) {
            HttpParamKind::Body
        } else {
            default_param_source(method)
        };
        let inferred_kind = infer_param_kind(
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
            kind: inferred_kind,
            optional: has_optional_annotation(&param.annotations),
            flatten: has_annotation(&param.annotations, "flatten"),
        };
        if matches!(projected.kind, HttpParamKind::Header) {
            validate_header_name(&projected.wire_name, &projected.name)?;
        }
        if matches!(projected.kind, HttpParamKind::Cookie) {
            validate_cookie_name(&projected.wire_name, &projected.name)?;
        }
        validate_projected_param(&op.ident, &projected, direction, route_path_names)?;
        distribute_param(
            projected,
            direction,
            &mut request_params,
            &mut response_params,
            &mut path_binding_count,
            &mut query_binding_count,
        );
    }

    Ok((
        request_params,
        response_params,
        path_binding_count,
        query_binding_count,
    ))
}

fn infer_param_kind(
    param: &hir::ParamDcl,
    direction: HttpParamDirection,
    binding: Option<&SourceBinding>,
    default_source: HttpParamKind,
    route_path_names: &[HashSet<String>],
    route_query_names: &[HashSet<String>],
) -> HttpParamKind {
    match direction {
        HttpParamDirection::Out | HttpParamDirection::InOut => binding
            .map(|value| value.source)
            .unwrap_or(HttpParamKind::Body),
        HttpParamDirection::In => binding.map(|value| value.source).unwrap_or_else(|| {
            if route_path_names
                .iter()
                .all(|set| set.contains(&param.declarator.0))
            {
                HttpParamKind::Path
            } else if route_query_names
                .iter()
                .any(|set| set.contains(&param.declarator.0))
            {
                HttpParamKind::Query
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
    response_params: &mut Vec<HttpParam>,
    path_binding_count: &mut HashMap<String, usize>,
    query_binding_count: &mut HashMap<String, usize>,
) {
    if matches!(
        direction,
        HttpParamDirection::In | HttpParamDirection::InOut
    ) {
        match projected.kind {
            HttpParamKind::Path => {
                *path_binding_count
                    .entry(projected.wire_name.clone())
                    .or_insert(0) += 1;
            }
            HttpParamKind::Query => {
                *query_binding_count
                    .entry(projected.wire_name.clone())
                    .or_insert(0) += 1;
            }
            HttpParamKind::Header | HttpParamKind::Cookie | HttpParamKind::Body => {}
        }
        request_params.push(projected.clone());
    }
    if matches!(
        direction,
        HttpParamDirection::Out | HttpParamDirection::InOut
    ) {
        response_params.push(projected.clone());
    }
}
