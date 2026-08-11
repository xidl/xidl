use crate::hir;
use std::collections::HashMap;

use super::annotation_parse::{parse_raw_params, render_const_expr, trim_quotes};

#[cfg(test)]
mod tests;

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

pub fn find_annotation<'a>(
    annotations: &'a [hir::Annotation],
    target: &str,
) -> Option<&'a hir::Annotation> {
    annotations.iter().find(|annotation| {
        annotation_name(annotation)
            .map(|name| name.eq_ignore_ascii_case(target))
            .unwrap_or(false)
    })
}

pub fn has_optional_annotation(annotations: &[hir::Annotation]) -> bool {
    annotations.iter().any(|annotation| {
        matches!(annotation, hir::Annotation::Optional { .. })
            || annotation_name(annotation)
                .map(|name| name.eq_ignore_ascii_case("optional"))
                .unwrap_or(false)
    })
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
        hir::AnnotationParams::Positional(values) => {
            let rendered = values
                .iter()
                .map(render_const_expr)
                .collect::<Vec<_>>()
                .join(", ");
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
) -> Option<String> {
    annotation_value(method_annotations, target)
        .or_else(|| annotation_value(interface_annotations, target))
        .and_then(|value| media_type_token_to_mime(&value).map(str::to_string))
}

pub(crate) fn annotation_value(annotations: &[hir::Annotation], target: &str) -> Option<String> {
    annotations.iter().find_map(|annotation| {
        let name = annotation_name(annotation)?;
        if !name.eq_ignore_ascii_case(target) {
            return None;
        }
        let params = annotation_params(annotation)?;
        let params = normalize_annotation_params(params);
        params.get("value").cloned().or_else(|| {
            media_type_param_keys(target)
                .iter()
                .find_map(|key| params.get(&key.to_ascii_lowercase()).cloned())
        })
    })
}

/// Translates a `@request`/`@response` format token into its canonical HTTP
/// media type, or `None` when the token is not one of `json`, `urlencoded`,
/// or `msgpack`.
pub(crate) fn media_type_token_to_mime(token: &str) -> Option<&'static str> {
    if token.eq_ignore_ascii_case("json") {
        Some("application/json")
    } else if token.eq_ignore_ascii_case("urlencoded") {
        Some("application/x-www-form-urlencoded")
    } else if token.eq_ignore_ascii_case("msgpack") {
        Some("application/msgpack")
    } else {
        None
    }
}

fn media_type_param_keys(target: &str) -> &'static [&'static str] {
    if target.eq_ignore_ascii_case("request") || target.eq_ignore_ascii_case("response") {
        &["format"]
    } else {
        &[]
    }
}
