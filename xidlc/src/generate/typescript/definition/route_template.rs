use std::collections::HashSet;

use crate::error::{IdlcError, IdlcResult};

use super::http::normalize_path;
use super::method::RouteTemplate;

pub(crate) fn path_param_is_catch_all(path: &str, name: &str) -> bool {
    path.contains(&format!("{{*{name}}}"))
}

pub(crate) fn split_query_template(path: &str) -> IdlcResult<(String, HashSet<String>)> {
    let mut query_params = HashSet::new();
    let Some(pos) = path.find("{?") else {
        return Ok((path.to_string(), query_params));
    };
    if !path.ends_with('}') {
        return Err(IdlcError::rpc(format!(
            "query template must terminate with '}}' in route '{path}'"
        )));
    }
    let tail = &path[pos + 2..path.len() - 1];
    if tail.trim().is_empty() {
        return Err(IdlcError::rpc(format!(
            "query template must include at least one variable in route '{path}'"
        )));
    }
    for name in tail.split(',').map(|value| value.trim()) {
        if name.is_empty() {
            return Err(IdlcError::rpc(format!(
                "query template contains empty variable name in route '{path}'"
            )));
        }
        query_params.insert(name.to_string());
    }
    Ok((path[..pos].to_string(), query_params))
}

pub(crate) fn validate_route_template(path: &str) -> IdlcResult<()> {
    let (path, _) = split_query_template(path)?;
    let mut start = 0usize;
    let mut catch_all_count = 0usize;
    while let Some(open_rel) = path[start..].find('{') {
        let open = start + open_rel;
        let close = path[open + 1..]
            .find('}')
            .map(|value| open + 1 + value)
            .ok_or_else(|| {
                IdlcError::rpc(format!("route template has unmatched '{{' in '{path}'"))
            })?;
        let token = &path[open + 1..close];
        let is_catch_all = token.starts_with('*');
        let name = token.strip_prefix('*').unwrap_or(token);
        if name.is_empty() {
            return Err(IdlcError::rpc(format!(
                "route template has empty path variable in '{path}'"
            )));
        }
        if is_catch_all {
            catch_all_count += 1;
            if catch_all_count > 1 {
                return Err(IdlcError::rpc(format!(
                    "route template contains more than one catch-all variable: '{path}'"
                )));
            }
            if close + 1 != path.len() {
                return Err(IdlcError::rpc(format!(
                    "catch-all variable must be at the end of route template: '{path}'"
                )));
            }
        }
        start = close + 1;
    }
    Ok(())
}

pub(crate) fn parse_route_template(path: &str) -> IdlcResult<RouteTemplate> {
    validate_route_template(path)?;
    let (path, query_params) = split_query_template(path)?;
    let normalized = normalize_path(&path);
    Ok(RouteTemplate {
        path: normalized.clone(),
        path_params: parse_path_params(&normalized),
        query_params,
    })
}

fn parse_path_params(path: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut buf = String::new();
    let mut in_param = false;
    for ch in path.chars() {
        match ch {
            '{' if !in_param => {
                in_param = true;
                buf.clear();
            }
            '}' if in_param => {
                if !buf.is_empty() {
                    out.insert(strip_path_param_prefix(&buf));
                }
                in_param = false;
            }
            _ if in_param => buf.push(ch),
            _ => {}
        }
    }
    out
}

fn strip_path_param_prefix(value: &str) -> String {
    value.strip_prefix('*').unwrap_or(value).to_string()
}
