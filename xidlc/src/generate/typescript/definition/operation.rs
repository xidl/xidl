use std::collections::HashMap;

use crate::error::{IdlcError, IdlcResult};
use crate::generate::utils::doc_lines_from_annotations;
use xidl_parser::hir;
use xidl_parser::rest_hir::{
    HttpInputBinding, HttpMethod, HttpOperation, HttpRequestBodyShape, semantics::HttpStreamKind,
};

use super::annotations::has_optional_annotation;
use super::method::{MethodInfo, ParamInfo, ReturnType};
use super::names::{method_struct_prefix, scoped_name, ts_ident};
use super::stream_validation::{validate_method, validate_target};

pub(crate) fn render_op(
    op: &hir::OpDcl,
    interface_name: &str,
    module_path: &[String],
    http_op: Option<&HttpOperation>,
) -> IdlcResult<MethodInfo> {
    let http_op = http_op.ok_or_else(|| {
        IdlcError::rpc(format!(
            "typescript: missing rest_hir projection for operation '{}'",
            op.ident
        ))
    })?;
    let stream = http_op.meta.stream;
    validate_target(&op.ident, stream)?;
    validate_method(&op.ident, stream.kind, http_op.meta.method)?;

    let is_server_stream = matches!(stream.kind, Some(HttpStreamKind::Server));
    let is_client_stream = matches!(stream.kind, Some(HttpStreamKind::Client));
    if is_client_stream
        && (!http_op.http.request.path.is_empty()
            || !http_op.http.request.query.is_empty()
            || !http_op.http.request.header.is_empty()
            || !http_op.http.request.cookie.is_empty())
    {
        return Err(IdlcError::rpc(format!(
            "@client_stream method '{}' currently supports body parameters only",
            op.ident
        )));
    }

    let ret = match &op.ty {
        hir::OpTypeSpec::Void => ReturnType::void(),
        hir::OpTypeSpec::TypeSpec(ty) => ReturnType::new(ty.clone()),
    };
    let raw_params = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);
    let by_name: HashMap<&str, &hir::ParamDcl> = raw_params
        .iter()
        .map(|param| (param.declarator.0.as_str(), param))
        .collect();

    let mut params = Vec::new();
    let mut output_params = Vec::new();
    for param in raw_params {
        let info = param_info_from_dcl(param, param.declarator.0.clone());
        match param.attr.as_ref().map(|value| value.0.as_str()) {
            Some("out") => output_params.push(info),
            Some("inout") => {
                output_params.push(info.clone());
                params.push(info);
            }
            _ => params.push(info),
        }
    }

    let path_params = map_bindings(&http_op.http.request.path, &by_name);
    let query_params = map_bindings(&http_op.http.request.query, &by_name);
    let header_params = map_bindings(&http_op.http.request.header, &by_name);
    let cookie_params = map_bindings(&http_op.http.request.cookie, &by_name);
    let body_params = map_body_params(&http_op.http.request.body.shape, &by_name);

    let path = http_op
        .meta
        .routes
        .first()
        .map(|route| route.path.clone())
        .unwrap_or_else(|| format!("/{}", op.ident));
    let request_name = (!params.is_empty())
        .then(|| format!("{}Request", method_struct_prefix(interface_name, &op.ident)));
    let request_schema_ref = request_name
        .as_ref()
        .map(|name| scoped_name(module_path, name));
    let response_name = (!output_params.is_empty()).then(|| {
        format!(
            "{}Response",
            method_struct_prefix(interface_name, &op.ident)
        )
    });

    Ok(MethodInfo {
        name: ts_ident(&op.ident),
        params,
        ret,
        response_name,
        http_method: http_method_name(http_op.meta.method).to_string(),
        path,
        request_name,
        request_schema_ref,
        path_params,
        query_params,
        header_params,
        cookie_params,
        body_params,
        output_params,
        is_server_stream,
        is_client_stream,
        doc: doc_lines_from_annotations(&op.annotations),
    })
}

fn param_info_from_dcl(param: &hir::ParamDcl, wire_name: String) -> ParamInfo {
    ParamInfo {
        name: ts_ident(&param.declarator.0),
        raw_name: param.declarator.0.clone(),
        wire_name,
        ty: param.ty.clone(),
        optional: has_optional_annotation(&param.annotations),
        doc: doc_lines_from_annotations(&param.annotations),
    }
}

fn map_bindings(
    bindings: &[HttpInputBinding],
    by_name: &HashMap<&str, &hir::ParamDcl>,
) -> Vec<ParamInfo> {
    bindings
        .iter()
        .map(|binding| match by_name.get(binding.source_param.as_str()) {
            Some(param) => param_info_from_dcl(param, binding.wire_name.clone()),
            None => ParamInfo {
                name: ts_ident(&binding.source_param),
                raw_name: binding.source_param.clone(),
                wire_name: binding.wire_name.clone(),
                ty: binding.ty.clone(),
                optional: binding.optional,
                doc: Vec::new(),
            },
        })
        .collect()
}

fn map_body_params(
    shape: &HttpRequestBodyShape,
    by_name: &HashMap<&str, &hir::ParamDcl>,
) -> Vec<ParamInfo> {
    match shape {
        HttpRequestBodyShape::Empty => Vec::new(),
        HttpRequestBodyShape::SingleValue {
            source_param,
            wire_name,
            ty,
            ..
        } => vec![body_param(source_param, wire_name, ty, false, by_name)],
        HttpRequestBodyShape::Stream {
            source_param,
            wire_name,
            item_ty,
            ..
        } => vec![body_param(source_param, wire_name, item_ty, false, by_name)],
        HttpRequestBodyShape::Object { fields } => fields
            .iter()
            .map(|field| {
                body_param(
                    &field.source_param,
                    &field.field_name,
                    &field.ty,
                    field.optional,
                    by_name,
                )
            })
            .collect(),
    }
}

fn body_param(
    source_param: &str,
    wire_name: &str,
    ty: &hir::TypeSpec,
    optional: bool,
    by_name: &HashMap<&str, &hir::ParamDcl>,
) -> ParamInfo {
    match by_name.get(source_param) {
        Some(param) => param_info_from_dcl(param, wire_name.to_string()),
        None => ParamInfo {
            name: ts_ident(source_param),
            raw_name: source_param.to_string(),
            wire_name: wire_name.to_string(),
            ty: ty.clone(),
            optional,
            doc: Vec::new(),
        },
    }
}

pub(crate) fn http_method_name(method: HttpMethod) -> &'static str {
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
