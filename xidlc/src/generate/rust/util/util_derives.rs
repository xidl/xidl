use std::collections::HashSet;
use xidl_parser::hir;

use super::util_annotation_parse::render_annotation_const_expr;
use super::util_annotations::{annotation_name_is_derive, annotation_params};

fn push_derive(out: &mut Vec<String>, seen: &mut HashSet<String>, value: &str) {
    let item = value.trim();
    if item.is_empty() {
        return;
    }
    if seen.insert(item.to_string()) {
        out.push(item.to_string());
    }
}

pub fn rust_derives_from_annotations(annotations: &[hir::Annotation]) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for annotation in annotations {
        if !annotation_name_is_derive(annotation) {
            continue;
        }
        let Some(params) = annotation_params(annotation) else {
            continue;
        };
        match params {
            hir::AnnotationParams::Raw(value) => {
                for item in value.split(',') {
                    push_derive(&mut out, &mut seen, item);
                }
            }
            hir::AnnotationParams::ConstExpr(expr) => {
                let rendered = render_annotation_const_expr(expr);
                for item in rendered.split(',') {
                    push_derive(&mut out, &mut seen, item);
                }
            }
            hir::AnnotationParams::Positional(values) => {
                for value in values {
                    let rendered = render_annotation_const_expr(value);
                    push_derive(&mut out, &mut seen, &rendered);
                }
            }
            hir::AnnotationParams::Params(values) => {
                for value in values {
                    push_derive(&mut out, &mut seen, &value.ident);
                }
            }
        }
    }
    out
}

pub fn rust_derives_from_annotations_with_extra(
    primary: &[hir::Annotation],
    extra: &[hir::Annotation],
) -> Vec<String> {
    let mut out = rust_derives_from_annotations(primary);
    let mut seen: HashSet<String> = out.iter().cloned().collect();
    for derive in rust_derives_from_annotations(extra) {
        if seen.insert(derive.clone()) {
            out.push(derive);
        }
    }

    // Add default derives if not present
    for default_derive in ["Debug", "Clone", "PartialEq", "Serialize", "Deserialize"] {
        if !seen.contains(default_derive)
            && !seen.contains(&format!("::serde::{default_derive}"))
            && seen.insert(default_derive.to_string())
        {
            out.push(default_derive.to_string());
        }
    }

    for value in &mut out {
        if value == "Serialize" {
            *value = "::serde::Serialize".to_string();
        }
        if value == "Deserialize" {
            *value = "::serde::Deserialize".to_string();
        }
    }
    out
}

pub struct RustDeriveInfo {
    pub all: Vec<String>,
    pub non_serde: Vec<String>,
    pub has_serde_serialize: bool,
    pub has_serde_deserialize: bool,
}

impl RustDeriveInfo {
    pub fn enable_serde_attrs(&self) -> bool {
        self.has_serde_serialize || self.has_serde_deserialize
    }
}

pub fn rust_derive_info_with_extra(
    primary: &[hir::Annotation],
    extra: &[hir::Annotation],
) -> RustDeriveInfo {
    let all = rust_derives_from_annotations_with_extra(primary, extra);
    let has_serde_serialize = all.iter().any(|value| value == "::serde::Serialize");
    let has_serde_deserialize = all.iter().any(|value| value == "::serde::Deserialize");
    let non_serde = all
        .iter()
        .filter(|value| {
            value.as_str() != "::serde::Serialize" && value.as_str() != "::serde::Deserialize"
        })
        .cloned()
        .collect();
    RustDeriveInfo {
        all,
        non_serde,
        has_serde_serialize,
        has_serde_deserialize,
    }
}
