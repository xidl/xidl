use std::collections::HashMap;
use xidl_parser::hir;

use super::util_annotation_parse::{
    parse_raw_annotation_params, render_annotation_const_expr, trim_annotation_quotes,
};

pub fn serde_rename_from_annotations(annotations: &[hir::Annotation]) -> Option<String> {
    for annotation in annotations {
        let Some(name) = annotation_name(annotation) else {
            continue;
        };
        if !name.eq_ignore_ascii_case("name") {
            continue;
        }
        let value = annotation_params(annotation)
            .map(normalize_annotation_params)
            .and_then(|params| {
                params
                    .get("value")
                    .cloned()
                    .or_else(|| params.get("name").cloned())
            });
        if value.is_some() {
            return value;
        }
    }
    None
}

pub fn rust_passthrough_attrs_from_annotations(annotations: &[hir::Annotation]) -> Vec<String> {
    let mut out = Vec::new();
    for annotation in annotations {
        if let Some(attr_name) = rust_passthrough_attr_name(annotation) {
            let rendered = annotation_params(annotation)
                .map(render_rust_passthrough_params)
                .unwrap_or_default();
            if rendered.is_empty() {
                out.push(attr_name);
            } else {
                out.push(format!("{attr_name}({rendered})"));
            }
        }
    }
    out
}

pub(crate) fn annotation_name_is_derive(annotation: &hir::Annotation) -> bool {
    match annotation {
        hir::Annotation::Builtin { name, .. } => name.eq_ignore_ascii_case("derive"),
        hir::Annotation::ScopedName { name, .. } => name
            .name
            .last()
            .map(|value| value.eq_ignore_ascii_case("derive"))
            .unwrap_or(false),
        _ => false,
    }
}

pub(crate) fn annotation_params(annotation: &hir::Annotation) -> Option<&hir::AnnotationParams> {
    match annotation {
        hir::Annotation::Builtin { params, .. } => params.as_ref(),
        hir::Annotation::ScopedName { params, .. } => params.as_ref(),
        _ => None,
    }
}

pub(crate) fn annotation_name(annotation: &hir::Annotation) -> Option<&str> {
    match annotation {
        hir::Annotation::Builtin { name, .. } => Some(name.as_str()),
        hir::Annotation::ScopedName { name, .. } => name.name.last().map(|value| value.as_str()),
        _ => None,
    }
}

fn rust_passthrough_attr_name(annotation: &hir::Annotation) -> Option<String> {
    let name = annotation_name(annotation)?;
    let lower = name.to_ascii_lowercase();
    let prefix = "rust-";
    if !lower.starts_with(prefix) {
        return None;
    }
    let attr = &name[prefix.len()..];
    if attr.is_empty() {
        None
    } else {
        Some(attr.to_string())
    }
}

fn normalize_annotation_params(params: &hir::AnnotationParams) -> HashMap<String, String> {
    let mut out = HashMap::new();
    match params {
        hir::AnnotationParams::Raw(value) => {
            for (key, value) in parse_raw_annotation_params(value) {
                out.insert(key.to_ascii_lowercase(), value);
            }
        }
        hir::AnnotationParams::Params(values) => {
            for value in values {
                let raw = value
                    .value
                    .as_ref()
                    .map(render_annotation_const_expr)
                    .unwrap_or_default();
                out.insert(
                    value.ident.to_ascii_lowercase(),
                    trim_annotation_quotes(&raw).unwrap_or(raw),
                );
            }
        }
        hir::AnnotationParams::ConstExpr(expr) => {
            let rendered = render_annotation_const_expr(expr);
            out.insert(
                "value".to_string(),
                trim_annotation_quotes(&rendered).unwrap_or(rendered),
            );
        }
    }
    out
}

fn render_rust_passthrough_params(params: &hir::AnnotationParams) -> String {
    match params {
        hir::AnnotationParams::Raw(value) => value.trim().to_string(),
        hir::AnnotationParams::ConstExpr(expr) => {
            render_annotation_const_expr(expr).trim().to_string()
        }
        hir::AnnotationParams::Params(values) => values
            .iter()
            .map(|value| {
                if let Some(expr) = &value.value {
                    format!(
                        "{} = {}",
                        value.ident,
                        render_annotation_const_expr(expr).trim()
                    )
                } else {
                    value.ident.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", "),
    }
}
