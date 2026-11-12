use crate::hir::{self, is_skipped};
use std::collections::HashMap;

pub(crate) mod sources;
use sources::ParameterSources;

use super::validate::{
    HttpParamDirection, param_direction, validate_cookie_name, validate_header_name,
    validate_projected_param,
};
use super::{HttpParam, HttpParamKind, RestHirResult};

#[cfg(test)]
mod tests;

#[allow(clippy::type_complexity)]
pub(super) fn project_params(
    op: &hir::OpDcl,
    sources: &ParameterSources,
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
    let mut distribution = ParamDistribution {
        request_params: &mut request_params,
        response_params: &mut response_params,
        path_binding_count: &mut path_binding_count,
        query_binding_count: &mut query_binding_count,
    };

    for param in params {
        if is_skipped(&param.annotations) {
            continue;
        }
        let direction = param_direction(param.attr.as_ref());
        let projected = sources.project(param)?;
        if matches!(projected.kind, HttpParamKind::Header) {
            validate_header_name(&projected.wire_name, &projected.name)?;
        }
        if matches!(projected.kind, HttpParamKind::Cookie) {
            validate_cookie_name(&projected.wire_name, &projected.name)?;
        }
        validate_projected_param(&op.ident, &projected, direction, &sources.route_path_names)?;
        distribution.distribute(projected, direction);
    }

    HttpParam::validate_unique_inputs(&request_params, &op.ident)?;
    Ok((
        request_params,
        response_params,
        path_binding_count,
        query_binding_count,
    ))
}

impl HttpParam {
    fn validate_unique_inputs(params: &[Self], operation: &str) -> RestHirResult<()> {
        for (index, first) in params.iter().enumerate() {
            for second in &params[index + 1..] {
                let same_name = if first.kind == HttpParamKind::Header {
                    first.wire_name.eq_ignore_ascii_case(&second.wire_name)
                } else {
                    first.wire_name == second.wire_name
                };
                if first.kind == second.kind && same_name {
                    return Err(format!(
                        "operation '{operation}' binds inputs '{}' and '{}' to the same {:?} wire name '{}'",
                        first.name, second.name, first.kind, first.wire_name
                    ));
                }
            }
        }
        Ok(())
    }
}

struct ParamDistribution<'a> {
    request_params: &'a mut Vec<HttpParam>,
    response_params: &'a mut Vec<HttpParam>,
    path_binding_count: &'a mut HashMap<String, usize>,
    query_binding_count: &'a mut HashMap<String, usize>,
}

impl ParamDistribution<'_> {
    fn distribute(&mut self, projected: HttpParam, direction: HttpParamDirection) {
        if matches!(
            direction,
            HttpParamDirection::In | HttpParamDirection::InOut
        ) {
            match projected.kind {
                HttpParamKind::Path => {
                    *self
                        .path_binding_count
                        .entry(projected.wire_name.clone())
                        .or_insert(0) += 1;
                }
                HttpParamKind::Query => {
                    *self
                        .query_binding_count
                        .entry(projected.wire_name.clone())
                        .or_insert(0) += 1;
                }
                HttpParamKind::Header | HttpParamKind::Cookie | HttpParamKind::Body => {}
            }
            self.request_params.push(projected.clone());
        }
        if matches!(
            direction,
            HttpParamDirection::Out | HttpParamDirection::InOut
        ) {
            self.response_params.push(projected.clone());
        }
    }
}
