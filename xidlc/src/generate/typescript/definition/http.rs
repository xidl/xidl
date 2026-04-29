use std::collections::HashSet;

use crate::error::{IdlcError, IdlcResult};
use xidl_parser::hir;

use super::annotations::{annotation_name, annotation_params, normalize_params};
use super::method::{HttpMethod, ParamDirection, ParamSource};

pub(crate) struct SourceBinding {
    pub(crate) source: ParamSource,
    pub(crate) bound_name: String,
}

pub(crate) fn route_from_annotations(
    annotations: &[hir::Annotation],
    default_method: HttpMethod,
) -> IdlcResult<(HttpMethod, Vec<String>)> {
    let mut method = None;
    let mut paths = Vec::new();
    for annotation in annotations {
        let Some(name) = annotation_name(annotation) else {
            continue;
        };
        if let Some(current) = method_from_annotation(annotation) {
            if let Some(prev) = method
                && prev != current
            {
                return Err(IdlcError::rpc(
                    "more than one HTTP verb annotation is not allowed on a method",
                ));
            }
            method = Some(current);
            if let Some(path) = annotation_params(annotation)
                .map(normalize_params)
                .and_then(|params| params.get("path").cloned())
            {
                paths.push(normalize_path(&path));
            }
            continue;
        }
        if name.eq_ignore_ascii_case("path")
            && let Some(path) = annotation_params(annotation)
                .map(normalize_params)
                .and_then(|params| {
                    params
                        .get("value")
                        .cloned()
                        .or_else(|| params.get("path").cloned())
                })
        {
            paths.push(normalize_path(&path));
        }
    }
    let mut dedup = HashSet::new();
    paths.retain(|path| dedup.insert(path.clone()));
    Ok((method.unwrap_or(default_method), paths))
}

fn method_from_annotation(annotation: &hir::Annotation) -> Option<HttpMethod> {
    match annotation_name(annotation)?.to_ascii_lowercase().as_str() {
        "get" => Some(HttpMethod::Get),
        "post" => Some(HttpMethod::Post),
        "put" => Some(HttpMethod::Put),
        "patch" => Some(HttpMethod::Patch),
        "delete" => Some(HttpMethod::Delete),
        "head" => Some(HttpMethod::Head),
        "options" => Some(HttpMethod::Options),
        _ => None,
    }
}

pub(crate) fn explicit_param_binding(param: &hir::ParamDcl) -> IdlcResult<Option<SourceBinding>> {
    let mut found = None;
    for annotation in &param.annotations {
        let Some(name) = annotation_name(annotation) else {
            continue;
        };
        let current = match name.to_ascii_lowercase().as_str() {
            "path" => Some(ParamSource::Path),
            "query" => Some(ParamSource::Query),
            "header" => Some(ParamSource::Header),
            "cookie" => Some(ParamSource::Cookie),
            _ => None,
        };
        let Some(current) = current else { continue };
        let bound_name = annotation_params(annotation)
            .map(normalize_params)
            .and_then(|params| params.get("value").cloned())
            .unwrap_or_else(|| param.declarator.0.clone());
        if matches!(current, ParamSource::Header) {
            validate_header_name(&bound_name, &param.declarator.0)?;
        }
        if matches!(current, ParamSource::Cookie) {
            validate_cookie_name(&bound_name, &param.declarator.0)?;
        }
        match found {
            None => {
                found = Some(SourceBinding {
                    source: current,
                    bound_name,
                })
            }
            Some(ref prev) if prev.source == current && prev.bound_name == bound_name => {}
            Some(_) => {
                return Err(IdlcError::rpc(format!(
                    "parameter '{}' has conflicting source annotations (@path/@query/@header/@cookie)",
                    param.declarator.0
                )));
            }
        }
    }
    Ok(found)
}

pub(crate) fn auto_default_method_path(op: &hir::OpDcl, method: HttpMethod) -> IdlcResult<String> {
    let params = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);
    let default_source = default_param_source(method);
    let mut path = normalize_path(&op.ident);
    for param in params {
        if matches!(param_direction(param.attr.as_ref()), ParamDirection::Out) {
            continue;
        }
        let (source, bound_name) = explicit_param_binding(param)?
            .map(|binding| (binding.source, binding.bound_name))
            .unwrap_or((default_source, param.declarator.0.clone()));
        if matches!(source, ParamSource::Path) {
            path.push_str(&format!("/{{{bound_name}}}"));
        }
    }
    Ok(path)
}

fn validate_header_name(bound_name: &str, param_name: &str) -> IdlcResult<()> {
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

fn validate_cookie_name(bound_name: &str, param_name: &str) -> IdlcResult<()> {
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

pub(crate) fn normalize_path(path: &str) -> String {
    let path = path.trim();
    let with_leading = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    let mut collapsed = String::with_capacity(with_leading.len());
    let mut prev_slash = false;
    for ch in with_leading.chars() {
        if ch == '/' {
            if !prev_slash {
                collapsed.push(ch);
            }
            prev_slash = true;
        } else {
            collapsed.push(ch);
            prev_slash = false;
        }
    }
    if collapsed.len() > 1 && collapsed.ends_with('/') {
        collapsed.pop();
    }
    if collapsed.is_empty() {
        "/".to_string()
    } else {
        collapsed
    }
}

pub(crate) fn path_name_in_all_routes(name: &str, route_sets: &[HashSet<String>]) -> bool {
    route_sets.iter().all(|set| set.contains(name))
}

pub(crate) fn validate_head_constraints(op: &hir::OpDcl, method: HttpMethod) -> IdlcResult<()> {
    if !matches!(method, HttpMethod::Head) {
        return Ok(());
    }
    if !matches!(op.ty, hir::OpTypeSpec::Void) {
        return Err(IdlcError::rpc(format!(
            "HEAD method '{}' must return void",
            op.ident
        )));
    }
    for param in op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[])
    {
        if matches!(
            param_direction(param.attr.as_ref()),
            ParamDirection::Out | ParamDirection::InOut
        ) {
            return Err(IdlcError::rpc(format!(
                "HEAD method '{}' cannot contain out/inout parameter '{}'",
                op.ident, param.declarator.0
            )));
        }
    }
    Ok(())
}

pub(crate) fn param_direction(attr: Option<&hir::ParamAttribute>) -> ParamDirection {
    match attr.map(|value| value.0.as_str()) {
        Some("out") => ParamDirection::Out,
        Some("inout") => ParamDirection::InOut,
        _ => ParamDirection::In,
    }
}

pub(crate) fn is_sequence_type(ty: &hir::TypeSpec) -> bool {
    matches!(ty, hir::TypeSpec::SequenceType(_))
}

pub(crate) fn default_param_source(method: HttpMethod) -> ParamSource {
    match method {
        HttpMethod::Get | HttpMethod::Delete | HttpMethod::Head | HttpMethod::Options => {
            ParamSource::Query
        }
        HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch => ParamSource::Body,
    }
}

pub(crate) fn method_http_code(method: HttpMethod) -> String {
    method_name(method).to_string()
}

pub(crate) fn method_name(method: HttpMethod) -> &'static str {
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
