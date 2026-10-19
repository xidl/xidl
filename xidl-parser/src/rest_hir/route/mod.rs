use super::semantics::SourceBinding;
use crate::hir;
use std::collections::{BTreeSet, HashSet};

use super::semantics::{annotation_name, annotation_params, normalize_annotation_params};
use super::validate::HttpParamDirection;
use super::{HttpMethod, HttpParamKind, HttpRoute, RestHirResult};

#[cfg(test)]
mod tests;

pub(super) fn route_from_annotations(
    annotations: &[hir::Annotation],
    default_method: HttpMethod,
) -> RestHirResult<(HttpMethod, Vec<String>)> {
    let mut method = None;
    let mut paths = Vec::new();
    for annotation in annotations {
        let Some(name) = annotation_name(annotation) else {
            continue;
        };
        if let Some(current) = method_from_annotation(annotation) {
            if let Some(previous) = method
                && previous != current
            {
                return Err(
                    "more than one HTTP verb annotation is not allowed on a method".to_string(),
                );
            }
            method = Some(current);
            if let Some(path) = annotation_params(annotation)
                .map(normalize_annotation_params)
                .and_then(|params| {
                    params
                        .get("value")
                        .cloned()
                        .or_else(|| params.get("path").cloned())
                })
            {
                paths.push(normalize_path(&path));
            }
            continue;
        }
        if name.eq_ignore_ascii_case("path")
            && let Some(path) = annotation_params(annotation)
                .map(normalize_annotation_params)
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
    let mut dedup = BTreeSet::new();
    paths.retain(|path| dedup.insert(path.clone()));
    Ok((method.unwrap_or(default_method), paths))
}

pub(super) fn auto_default_method_path(
    op: &hir::OpDcl,
    method: HttpMethod,
) -> RestHirResult<String> {
    let mut path = normalize_path(&op.ident);
    let params = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);
    for param in params {
        if matches!(
            super::validate::param_direction(param.attr.as_ref()),
            HttpParamDirection::Out
        ) {
            continue;
        }
        let binding = SourceBinding::read(&param.annotations, &param.declarator.0)?;
        let source = binding
            .as_ref()
            .map(|value| value.source)
            .unwrap_or(super::validate::default_param_source(method));
        let bound_name = binding.as_ref().map_or_else(
            || hir::field_rename(&param.annotations).unwrap_or_else(|| param.declarator.0.clone()),
            |binding| binding.wire_name(&param.declarator.0, &param.annotations, &[]),
        );
        if matches!(source, HttpParamKind::Path) {
            path.push('/');
            path.push('{');
            path.push_str(&bound_name);
            path.push('}');
        }
    }
    Ok(path)
}

pub(super) fn parse_route_template(path: &str) -> RestHirResult<HttpRoute> {
    let (path, query_params) = split_query_template(path)?;
    validate_route_template(&path)?;
    let path = normalize_path(&path);
    let mut path_params = parse_path_params(&path).into_iter().collect::<Vec<_>>();
    let mut query_params = query_params.into_iter().collect::<Vec<_>>();
    path_params.sort();
    query_params.sort();
    Ok(HttpRoute {
        path,
        path_params,
        query_params,
    })
}

fn split_query_template(path: &str) -> RestHirResult<(String, HashSet<String>)> {
    let mut query_params = HashSet::new();
    if let Some(pos) = path.find("{?") {
        if !path.ends_with('}') {
            return Err(format!(
                "query template must terminate with '}}' in route '{path}'"
            ));
        }
        let tail = &path[pos + 2..path.len() - 1];
        if tail.trim().is_empty() {
            return Err(format!(
                "query template must include at least one variable in route '{path}'"
            ));
        }
        for name in tail.split(',').map(str::trim) {
            if name.is_empty() {
                return Err(format!(
                    "query template contains empty variable name in route '{path}'"
                ));
            }
            query_params.insert(name.to_string());
        }
        Ok((path[..pos].to_string(), query_params))
    } else {
        Ok((path.to_string(), query_params))
    }
}

fn validate_route_template(path: &str) -> RestHirResult<()> {
    let mut start = 0usize;
    let mut catch_all_count = 0usize;
    while let Some(open_rel) = path[start..].find('{') {
        let open = start + open_rel;
        let close = path[open + 1..]
            .find('}')
            .map(|value| open + 1 + value)
            .ok_or_else(|| format!("route template has unmatched '{{' in '{path}'"))?;
        let token = &path[open + 1..close];
        let is_catch_all = token.starts_with('*');
        let name = token.strip_prefix('*').unwrap_or(token);
        if name.is_empty() {
            return Err(format!(
                "route template has empty path variable in '{path}'"
            ));
        }
        if is_catch_all {
            catch_all_count += 1;
            if catch_all_count > 1 {
                return Err(format!(
                    "route template contains more than one catch-all variable: '{path}'"
                ));
            }
            if close + 1 != path.len() {
                return Err(format!(
                    "catch-all variable must be at the end of route template: '{path}'"
                ));
            }
        }
        start = close + 1;
    }
    Ok(())
}

fn parse_path_params(path: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut in_param = false;
    let mut current = String::new();
    for ch in path.chars() {
        match ch {
            '{' if !in_param => {
                in_param = true;
                current.clear();
            }
            '}' if in_param => {
                if !current.is_empty() {
                    out.insert(current.trim_start_matches('*').to_string());
                }
                in_param = false;
            }
            _ if in_param => current.push(ch),
            _ => {}
        }
    }
    out
}

pub(super) fn normalize_path(path: &str) -> String {
    let path = path.trim();
    let with_leading = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    let mut out = String::new();
    let mut prev_slash = false;
    for ch in with_leading.chars() {
        if ch == '/' {
            if !prev_slash {
                out.push(ch);
            }
            prev_slash = true;
        } else {
            prev_slash = false;
            out.push(ch);
        }
    }
    if out.len() > 1 && out.ends_with('/') {
        out.pop();
    }
    if out.is_empty() { "/".to_string() } else { out }
}

fn method_from_annotation(annotation: &hir::Annotation) -> Option<HttpMethod> {
    let name = annotation_name(annotation)?;
    use crate::lint::AnnotationKind;
    match AnnotationKind::from_name(name)? {
        AnnotationKind::Get => Some(HttpMethod::Get),
        AnnotationKind::Post => Some(HttpMethod::Post),
        AnnotationKind::Put => Some(HttpMethod::Put),
        AnnotationKind::Patch => Some(HttpMethod::Patch),
        AnnotationKind::Delete => Some(HttpMethod::Delete),
        AnnotationKind::Head => Some(HttpMethod::Head),
        AnnotationKind::Options => Some(HttpMethod::Options),
        _ => None,
    }
}

pub(super) fn readonly_attr_names(spec: &hir::ReadonlyAttrSpec) -> Vec<String> {
    spec.declarator
        .declarators()
        .iter()
        .map(|decl| decl.0.clone())
        .collect()
}

pub(super) fn attribute_path(name: &str) -> String {
    format!("/attribute/{name}")
}

pub(super) fn default_path(
    module_path: &[String],
    interface_name: &str,
    method_name: &str,
) -> String {
    let mut parts = module_path.to_vec();
    parts.push(interface_name.to_string());
    parts.push(method_name.to_string());
    format!("/{}", parts.join("/"))
}

pub(super) fn operation_id(
    module_path: &[String],
    interface_name: &str,
    method_name: &str,
) -> String {
    let mut parts = module_path.to_vec();
    parts.push(interface_name.to_string());
    parts.push(method_name.to_string());
    parts.join(".")
}
