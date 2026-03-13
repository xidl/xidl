use std::collections::{BTreeSet, HashMap};

use jiff::{Timestamp, civil, tz::TimeZone};
use xidl_parser::hir;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeprecatedInfo {
    pub deprecated: bool,
    pub since: Option<String>,
    pub after: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpApiKeyLocation {
    Header,
    Query,
    Cookie,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpSecurityRequirement {
    HttpBasic,
    HttpBearer,
    ApiKey {
        location: HttpApiKeyLocation,
        name: String,
    },
    OAuth2 {
        scopes: Vec<String>,
    },
}

pub fn annotation_name(annotation: &hir::Annotation) -> Option<&str> {
    match annotation {
        hir::Annotation::Builtin { name, .. } => Some(name.as_str()),
        hir::Annotation::ScopedName { name, .. } => name.name.last().map(|value| value.as_str()),
        _ => None,
    }
}

pub fn annotation_params(annotation: &hir::Annotation) -> Option<&hir::AnnotationParams> {
    match annotation {
        hir::Annotation::Builtin { params, .. } => params.as_ref(),
        hir::Annotation::ScopedName { params, .. } => params.as_ref(),
        _ => None,
    }
}

pub fn has_annotation(annotations: &[hir::Annotation], target: &str) -> bool {
    annotations.iter().any(|annotation| {
        annotation_name(annotation)
            .map(|name| name.eq_ignore_ascii_case(target))
            .unwrap_or(false)
    })
}

pub fn has_optional_annotation(annotations: &[hir::Annotation]) -> bool {
    has_annotation(annotations, "optional")
}

pub fn normalize_annotation_params(params: &hir::AnnotationParams) -> HashMap<String, String> {
    let mut out = HashMap::new();
    match params {
        hir::AnnotationParams::Raw(value) => {
            for (key, value) in parse_raw_params(value) {
                out.insert(key.to_ascii_lowercase(), value);
            }
        }
        hir::AnnotationParams::Params(values) => {
            for value in values {
                let raw = value
                    .value
                    .as_ref()
                    .map(render_const_expr)
                    .unwrap_or_default();
                out.insert(
                    value.ident.to_ascii_lowercase(),
                    trim_quotes(&raw).unwrap_or(raw),
                );
            }
        }
        hir::AnnotationParams::ConstExpr(expr) => {
            let rendered = render_const_expr(expr);
            out.insert(
                "value".to_string(),
                trim_quotes(&rendered).unwrap_or(rendered),
            );
        }
    }
    out
}

pub fn effective_media_type(
    interface_annotations: &[hir::Annotation],
    method_annotations: &[hir::Annotation],
    target: &str,
) -> String {
    annotation_value(method_annotations, target)
        .or_else(|| annotation_value(interface_annotations, target))
        .unwrap_or_else(|| "application/json".to_string())
}

pub fn deprecated_info(annotations: &[hir::Annotation]) -> Result<Option<DeprecatedInfo>, String> {
    let annotation = annotations
        .iter()
        .find(|annotation| annotation_name(annotation)
            .map(|name| name.eq_ignore_ascii_case("deprecated"))
            .unwrap_or(false));
    let Some(annotation) = annotation else {
        return Ok(None);
    };
    let mut since = None;
    let mut after = None;
    if let Some(params) = annotation_params(annotation) {
        let params = normalize_annotation_params(params);
        if let Some(value) = params.get("value") {
            since = Some(normalize_deprecated_timestamp(value, false)?);
        }
        if let Some(value) = params.get("since") {
            since = Some(normalize_deprecated_timestamp(value, false)?);
        }
        if let Some(value) = params.get("after") {
            after = Some(normalize_deprecated_timestamp(value, true)?);
        }
    }
    if let (Some(since), Some(after)) = (&since, &after) {
        let since_ts: Timestamp = since
            .parse()
            .map_err(|_| format!("invalid @deprecated(since) timestamp '{since}'"))?;
        let after_ts: Timestamp = after
            .parse()
            .map_err(|_| format!("invalid @deprecated(after) timestamp '{after}'"))?;
        if since_ts > after_ts {
            return Err("@deprecated(since=..., after=...) requires since <= after".to_string());
        }
    }
    Ok(Some(DeprecatedInfo {
        deprecated: true,
        since,
        after,
    }))
}

pub fn effective_security(
    interface_annotations: &[hir::Annotation],
    method_annotations: &[hir::Annotation],
) -> Result<Option<Vec<HttpSecurityRequirement>>, String> {
    let method_security = collect_security(method_annotations)?;
    if method_security.explicit_none {
        return Ok(Some(Vec::new()));
    }
    if !method_security.requirements.is_empty() {
        return Ok(Some(method_security.requirements));
    }
    let interface_security = collect_security(interface_annotations)?;
    if interface_security.explicit_none {
        return Ok(Some(Vec::new()));
    }
    if interface_security.requirements.is_empty() {
        Ok(None)
    } else {
        Ok(Some(interface_security.requirements))
    }
}

pub fn validate_http_annotations(
    target: &str,
    annotations: &[hir::Annotation],
) -> Result<(), String> {
    let _ = deprecated_info(annotations)
        .map_err(|err| format!("{target}: {err}"))?;
    let _ = collect_security(annotations)
        .map_err(|err| format!("{target}: {err}"))?;
    Ok(())
}

fn annotation_value(annotations: &[hir::Annotation], target: &str) -> Option<String> {
    annotations.iter().find_map(|annotation| {
        let name = annotation_name(annotation)?;
        if !name.eq_ignore_ascii_case(target) {
            return None;
        }
        let params = annotation_params(annotation)?;
        let params = normalize_annotation_params(params);
        params
            .get("value")
            .cloned()
            .or_else(|| params.get(target).cloned())
    })
}

fn normalize_deprecated_timestamp(value: &str, end_of_day: bool) -> Result<String, String> {
    if let Ok(ts) = value.parse::<Timestamp>() {
        return Ok(ts.to_zoned(TimeZone::UTC).timestamp().to_string());
    }
    let date: civil::Date = value
        .parse()
        .map_err(|_| format!("invalid @deprecated timestamp literal '{value}'"))?;
    let dt = if end_of_day {
        date.at(23, 59, 59, 0)
    } else {
        date.to_datetime(civil::Time::midnight())
    };
    let zoned = dt.to_zoned(TimeZone::UTC).map_err(|err| err.to_string())?;
    Ok(zoned.timestamp().to_string())
}

struct SecurityCollection {
    explicit_none: bool,
    requirements: Vec<HttpSecurityRequirement>,
}

fn collect_security(annotations: &[hir::Annotation]) -> Result<SecurityCollection, String> {
    let mut explicit_none = false;
    let mut requirements = Vec::new();
    let mut singleton_names = BTreeSet::new();

    for annotation in annotations {
        let Some(name) = annotation_name(annotation) else {
            continue;
        };
        if name.eq_ignore_ascii_case("no-security") {
            explicit_none = true;
            continue;
        }
        let requirement = if name.eq_ignore_ascii_case("http-basic") {
            ensure_singleton(&mut singleton_names, "http-basic")?;
            Some(HttpSecurityRequirement::HttpBasic)
        } else if name.eq_ignore_ascii_case("http-bearer") {
            ensure_singleton(&mut singleton_names, "http-bearer")?;
            Some(HttpSecurityRequirement::HttpBearer)
        } else if name.eq_ignore_ascii_case("api-key") {
            Some(parse_api_key(annotation)?)
        } else if name.eq_ignore_ascii_case("oauth2") {
            Some(parse_oauth2(annotation))
        } else {
            None
        };
        if let Some(requirement) = requirement {
            requirements.push(requirement);
        }
    }

    if explicit_none && !requirements.is_empty() {
        return Err("@no-security cannot be combined with other security annotations".to_string());
    }

    Ok(SecurityCollection {
        explicit_none,
        requirements,
    })
}

fn ensure_singleton(names: &mut BTreeSet<&'static str>, value: &'static str) -> Result<(), String> {
    if !names.insert(value) {
        return Err(format!("duplicate @{value} annotation"));
    }
    Ok(())
}

fn parse_api_key(annotation: &hir::Annotation) -> Result<HttpSecurityRequirement, String> {
    let params = annotation_params(annotation)
        .ok_or_else(|| "@api-key requires in=... and name=...".to_string())?;
    let params = normalize_annotation_params(params);
    let location = match params.get("in").map(|value| value.to_ascii_lowercase()) {
        Some(value) if value == "header" => HttpApiKeyLocation::Header,
        Some(value) if value == "query" => HttpApiKeyLocation::Query,
        Some(value) if value == "cookie" => HttpApiKeyLocation::Cookie,
        Some(value) => {
            return Err(format!(
                "@api-key(in=...) must be one of header|query|cookie, got '{value}'"
            ));
        }
        None => return Err("@api-key requires non-empty in=...".to_string()),
    };
    let name = params
        .get("name")
        .cloned()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "@api-key requires non-empty name=...".to_string())?;
    Ok(HttpSecurityRequirement::ApiKey { location, name })
}

fn parse_oauth2(annotation: &hir::Annotation) -> HttpSecurityRequirement {
    let params = annotation_params(annotation)
        .map(normalize_annotation_params)
        .unwrap_or_default();
    let scopes = params
        .get("scopes")
        .map(|value| parse_string_array(value))
        .unwrap_or_default();
    HttpSecurityRequirement::OAuth2 { scopes }
}

fn parse_raw_params(raw: &str) -> Vec<(String, String)> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    if !trimmed.contains('=') {
        return vec![("value".to_string(), trim_quotes(trimmed).unwrap_or_else(|| trimmed.to_string()))];
    }
    split_top_level(trimmed, ',')
        .into_iter()
        .filter_map(|part| {
            let (key, value) = part.split_once('=')?;
            let key = key.trim();
            if key.is_empty() {
                return None;
            }
            let value = value.trim();
            Some((
                key.to_string(),
                trim_quotes(value).unwrap_or_else(|| value.to_string()),
            ))
        })
        .collect()
}

fn parse_string_array(raw: &str) -> Vec<String> {
    let trimmed = raw.trim();
    let inner = trimmed
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(trimmed);
    split_top_level(inner, ',')
        .into_iter()
        .map(|value| trim_quotes(value.trim()).unwrap_or_else(|| value.trim().to_string()))
        .filter(|value| !value.is_empty())
        .collect()
}

fn split_top_level(raw: &str, separator: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut bracket_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut escaped = false;
    for ch in raw.chars() {
        if let Some(q) = quote {
            current.push(ch);
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == q {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' => {
                quote = Some(ch);
                current.push(ch);
            }
            '[' => {
                bracket_depth += 1;
                current.push(ch);
            }
            ']' => {
                bracket_depth = bracket_depth.saturating_sub(1);
                current.push(ch);
            }
            '(' => {
                paren_depth += 1;
                current.push(ch);
            }
            ')' => {
                paren_depth = paren_depth.saturating_sub(1);
                current.push(ch);
            }
            _ if ch == separator && bracket_depth == 0 && paren_depth == 0 => {
                parts.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }
    parts
}

fn trim_quotes(value: &str) -> Option<String> {
    let value = value.trim();
    if value.len() >= 2 {
        let first = value.chars().next().unwrap();
        let last = value.chars().last().unwrap();
        if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
            return Some(value[1..value.len() - 1].to_string());
        }
    }
    None
}

fn render_const_expr(expr: &hir::ConstExpr) -> String {
    crate::generate::render_const_expr(
        expr,
        &crate::generate::rust::util::rust_scoped_name,
        &crate::generate::rust::util::rust_literal,
    )
}
