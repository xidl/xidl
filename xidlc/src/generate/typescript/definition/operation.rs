use std::collections::{HashMap, HashSet};

use crate::error::{IdlcError, IdlcResult};
use crate::generate::utils::doc_lines_from_annotations;
use xidl_parser::hir;
use xidl_parser::http_hir::semantics::{HttpStreamKind, http_stream_config};

use super::annotations::has_optional_annotation;
use super::http::{
    auto_default_method_path, default_param_source, explicit_param_binding, method_http_code,
    param_direction, path_name_in_all_routes, validate_head_constraints,
};
use super::method::{HttpMethod, MethodInfo, ParamDirection, ParamInfo, ParamSource, ReturnType};
use super::names::{method_struct_prefix, scoped_name, ts_ident};
use super::route_template::parse_route_template;
use super::stream_validation::{validate_method, validate_target};

pub(crate) fn render_op(
    op: &hir::OpDcl,
    interface_name: &str,
    module_path: &[String],
) -> IdlcResult<MethodInfo> {
    let stream = http_stream_config(&op.annotations).map_err(IdlcError::rpc)?;
    validate_target(&op.ident, stream)?;
    let is_server_stream = matches!(stream.kind, Some(HttpStreamKind::Server));
    let is_client_stream = matches!(stream.kind, Some(HttpStreamKind::Client));
    let ret = match &op.ty {
        hir::OpTypeSpec::Void => ReturnType::void(),
        hir::OpTypeSpec::TypeSpec(ty) => ReturnType::new(ty.clone()),
    };
    let params = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);
    let default_method = if is_server_stream {
        HttpMethod::Get
    } else {
        HttpMethod::Post
    };
    let (method, mut paths) = super::http::route_from_annotations(&op.annotations, default_method)?;
    validate_method(&op.ident, stream.kind, method)?;
    if paths.is_empty() {
        paths.push(auto_default_method_path(op, method)?);
    }
    validate_head_constraints(op, method)?;
    let route_templates = paths
        .iter()
        .map(|value| parse_route_template(value))
        .collect::<IdlcResult<Vec<_>>>()?;
    let path = route_templates
        .first()
        .map(|value| value.path.clone())
        .unwrap_or_else(|| format!("/{}", op.ident));
    let path_param_sets: Vec<HashSet<String>> = route_templates
        .iter()
        .map(|value| value.path_params.clone())
        .collect();
    let all_path_param_names = path_param_sets
        .iter()
        .flat_map(|set| set.iter().cloned())
        .collect::<HashSet<_>>();
    let all_query_template_names = route_templates
        .iter()
        .flat_map(|value| value.query_params.iter().cloned())
        .collect::<HashSet<_>>();
    let default_source = default_param_source(method);
    let mut buckets = ParamBuckets::default();
    let mut path_binding_count = HashMap::<String, usize>::new();
    let mut query_binding_count = HashMap::<String, usize>::new();
    for param in params {
        classify_param(
            param,
            &all_path_param_names,
            &all_query_template_names,
            &path_param_sets,
            default_source,
            &mut path_binding_count,
            &mut query_binding_count,
            &mut buckets,
            &op.ident,
        )?;
    }
    validate_route_bindings(&path_param_sets, &path_binding_count, "path", &op.ident)?;
    validate_route_bindings(
        &route_templates
            .iter()
            .map(|value| value.query_params.clone())
            .collect::<Vec<_>>(),
        &query_binding_count,
        "query",
        &op.ident,
    )?;
    if is_client_stream
        && (!buckets.path_params.is_empty()
            || !buckets.query_params.is_empty()
            || !buckets.header_params.is_empty()
            || !buckets.cookie_params.is_empty())
    {
        return Err(IdlcError::rpc(format!(
            "@client_stream method '{}' currently supports body parameters only",
            op.ident
        )));
    }
    let request_name = (!buckets.params.is_empty())
        .then(|| format!("{}Request", method_struct_prefix(interface_name, &op.ident)));
    let request_schema_ref = request_name.as_ref().map(|name| {
        let full = scoped_name(module_path, name);
        format!("zodSchemas.{full}Schema")
    });
    let response_name = (!buckets.output_params.is_empty()).then(|| {
        format!(
            "{}Response",
            method_struct_prefix(interface_name, &op.ident)
        )
    });
    Ok(MethodInfo {
        name: ts_ident(&op.ident),
        params: buckets.params,
        ret,
        response_name,
        http_method: method_http_code(method),
        path,
        request_name,
        request_schema_ref,
        path_params: buckets.path_params,
        query_params: buckets.query_params,
        header_params: buckets.header_params,
        cookie_params: buckets.cookie_params,
        body_params: buckets.body_params,
        output_params: buckets.output_params,
        is_server_stream,
        is_client_stream,
        doc: doc_lines_from_annotations(&op.annotations),
    })
}

#[derive(Default)]
struct ParamBuckets {
    params: Vec<ParamInfo>,
    path_params: Vec<ParamInfo>,
    query_params: Vec<ParamInfo>,
    header_params: Vec<ParamInfo>,
    cookie_params: Vec<ParamInfo>,
    body_params: Vec<ParamInfo>,
    output_params: Vec<ParamInfo>,
}

#[allow(clippy::too_many_arguments)]
fn classify_param(
    param: &hir::ParamDcl,
    all_path_param_names: &HashSet<String>,
    all_query_template_names: &HashSet<String>,
    path_param_sets: &[HashSet<String>],
    default_source: ParamSource,
    path_binding_count: &mut HashMap<String, usize>,
    query_binding_count: &mut HashMap<String, usize>,
    buckets: &mut ParamBuckets,
    op_ident: &str,
) -> IdlcResult<()> {
    let binding = explicit_param_binding(param)?;
    let (source, wire_name) = match binding {
        Some(binding) => (binding.source, binding.bound_name),
        None if all_path_param_names.contains(&param.declarator.0) => {
            (ParamSource::Path, param.declarator.0.clone())
        }
        None if all_query_template_names.contains(&param.declarator.0) => {
            (ParamSource::Query, param.declarator.0.clone())
        }
        None => (default_source, param.declarator.0.clone()),
    };
    validate_binding(
        source,
        &wire_name,
        &param.declarator.0,
        all_path_param_names,
        path_param_sets,
        op_ident,
    )?;
    let info = ParamInfo {
        name: ts_ident(&param.declarator.0),
        raw_name: param.declarator.0.clone(),
        wire_name,
        ty: param.ty.clone(),
        optional: has_optional_annotation(&param.annotations),
        doc: doc_lines_from_annotations(&param.annotations),
    };
    let direction = param_direction(param.attr.as_ref());
    if matches!(direction, ParamDirection::Out | ParamDirection::InOut) {
        buckets.output_params.push(info.clone());
    }
    if matches!(direction, ParamDirection::Out) {
        return Ok(());
    }
    buckets.params.push(info.clone());
    match source {
        ParamSource::Path => push_param(info, &mut buckets.path_params, path_binding_count),
        ParamSource::Query => push_param(info, &mut buckets.query_params, query_binding_count),
        ParamSource::Header => buckets.header_params.push(info),
        ParamSource::Cookie => buckets.cookie_params.push(info),
        ParamSource::Body => buckets.body_params.push(info),
    }
    Ok(())
}

fn validate_binding(
    source: ParamSource,
    wire_name: &str,
    param_name: &str,
    all_path_param_names: &HashSet<String>,
    path_param_sets: &[HashSet<String>],
    op_ident: &str,
) -> IdlcResult<()> {
    if matches!(source, ParamSource::Path) && !all_path_param_names.contains(wire_name) {
        return Err(IdlcError::rpc(format!(
            "parameter '{}' is annotated with @path but '{}' is not present in route template of method '{}'",
            param_name, wire_name, op_ident
        )));
    }
    if matches!(source, ParamSource::Path) && !path_name_in_all_routes(wire_name, path_param_sets) {
        return Err(IdlcError::rpc(format!(
            "parameter '{}' is bound to path variable '{}' but it is not present in every route template of method '{}'",
            param_name, wire_name, op_ident
        )));
    }
    Ok(())
}

fn push_param(info: ParamInfo, target: &mut Vec<ParamInfo>, counts: &mut HashMap<String, usize>) {
    *counts.entry(info.wire_name.clone()).or_insert(0) += 1;
    target.push(info);
}

fn validate_route_bindings(
    route_sets: &[HashSet<String>],
    binding_count: &HashMap<String, usize>,
    kind: &str,
    op_ident: &str,
) -> IdlcResult<()> {
    for route_params in route_sets {
        for route_param in route_params {
            match binding_count.get(route_param).copied().unwrap_or(0) {
                0 => {
                    return Err(IdlcError::rpc(format!(
                        "{kind} template variable '{}' has no matching request-side {kind} parameter in method '{}'",
                        route_param, op_ident
                    )));
                }
                1 => {}
                _ => {
                    return Err(IdlcError::rpc(format!(
                        "{kind} template variable '{}' is bound by multiple request-side {kind} parameters in method '{}'",
                        route_param, op_ident
                    )));
                }
            }
        }
    }
    Ok(())
}
