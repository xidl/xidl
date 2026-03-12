use crate::error::IdlcResult;
use crate::generate::rust::{RustRender, RustRenderOutput, RustRenderer};
use serde_json::json;
use std::collections::HashMap;
use xidl_parser::hir;

impl RustRender for hir::EnumDcl {
    fn render(&self, renderer: &RustRenderer) -> IdlcResult<RustRenderOutput> {
        let members = self
            .member
            .iter()
            .map(|member| {
                let rust_name = crate::generate::rust::util::rust_ident(&member.ident);
                let rename = field_rename_raw(&member.annotations);
                json!({
                    "name": rust_name,
                    "serde_rename": rename,
                })
            })
            .collect::<Vec<_>>();
        let derive = crate::generate::rust::util::rust_derives_from_annotations_with_extra(
            &self.annotations,
            &self.annotations,
        );
        let has_serde_serialize = derive.iter().any(|d| d == "::serde::Serialize");
        let has_serde_deserialize = derive.iter().any(|d| d == "::serde::Deserialize");
        let ctx = json!({
            "ident": crate::generate::rust::util::rust_ident(&self.ident),
            "members": members,
            "has_serde_serialize": has_serde_serialize,
            "has_serde_deserialize": has_serde_deserialize,
            "typeobject_path": renderer.typeobject_path(),
        });
        let rendered = renderer.render_template("enum.rs.j2", &ctx)?;
        Ok(RustRenderOutput::default().push_source(rendered))
    }
}

fn field_rename_raw(annotations: &[hir::Annotation]) -> Option<String> {
    for annotation in annotations {
        let Some(name) = annotation_name(annotation) else {
            continue;
        };
        if !name.eq_ignore_ascii_case("name") {
            continue;
        }
        let value = annotation_params(annotation)
            .map(normalize_params)
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

fn annotation_name(annotation: &hir::Annotation) -> Option<&str> {
    match annotation {
        hir::Annotation::Builtin { name, .. } => Some(name.as_str()),
        hir::Annotation::ScopedName { name, .. } => name.name.last().map(|value| value.as_str()),
        _ => None,
    }
}

fn annotation_params(annotation: &hir::Annotation) -> Option<&hir::AnnotationParams> {
    match annotation {
        hir::Annotation::Builtin { params, .. } => params.as_ref(),
        hir::Annotation::ScopedName { params, .. } => params.as_ref(),
        _ => None,
    }
}

fn normalize_params(params: &hir::AnnotationParams) -> HashMap<String, String> {
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

fn render_const_expr(expr: &hir::ConstExpr) -> String {
    crate::generate::render_const_expr(
        expr,
        &crate::generate::rust::util::rust_scoped_name,
        &crate::generate::rust::util::rust_literal,
    )
}

fn trim_quotes(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.len() >= 2 {
        let bytes = raw.as_bytes();
        let first = bytes[0] as char;
        let last = bytes[raw.len() - 1] as char;
        if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
            return Some(raw[1..raw.len() - 1].to_string());
        }
    }
    None
}

fn parse_raw_params(raw: &str) -> Vec<(String, String)> {
    let mut parts = Vec::new();
    let mut buf = String::new();
    let mut quote = None;
    let mut iter = raw.chars().peekable();
    while let Some(ch) = iter.next() {
        match ch {
            '"' | '\'' => {
                if quote == Some(ch) {
                    quote = None;
                } else if quote.is_none() {
                    quote = Some(ch);
                }
                buf.push(ch);
            }
            ',' if quote.is_none() => {
                push_raw_param(&mut parts, &buf);
                buf.clear();
            }
            _ => buf.push(ch),
        }
    }
    push_raw_param(&mut parts, &buf);
    parts
}

fn push_raw_param(parts: &mut Vec<(String, String)>, raw: &str) {
    let raw = raw.trim();
    if raw.is_empty() {
        return;
    }
    if let Some((key, value)) = raw.split_once('=') {
        let key = key.trim();
        let value = value.trim();
        if !key.is_empty() {
            let value = trim_quotes(value).unwrap_or_else(|| value.to_string());
            parts.push((key.to_string(), value));
        }
    } else {
        let value = trim_quotes(raw).unwrap_or_else(|| raw.to_string());
        parts.push(("value".to_string(), value));
    }
}
