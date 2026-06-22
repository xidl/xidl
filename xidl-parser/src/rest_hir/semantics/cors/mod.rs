use crate::hir;
use serde::{Deserialize, Serialize};

use super::annotations::{annotation_name, annotation_params};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpCorsProfile {
    Any,
    Origins(Vec<String>),
}

pub fn effective_cors(
    interface_annotations: &[hir::Annotation],
    method_annotations: &[hir::Annotation],
) -> Result<Option<HttpCorsProfile>, String> {
    collect_cors(method_annotations)?.map_or_else(
        || collect_cors(interface_annotations),
        |profile| Ok(Some(profile)),
    )
}

pub(crate) fn collect_cors(
    annotations: &[hir::Annotation],
) -> Result<Option<HttpCorsProfile>, String> {
    let mut matches = annotations.iter().filter(|annotation| {
        annotation_name(annotation)
            .map(|name| name.eq_ignore_ascii_case("cors"))
            .unwrap_or(false)
    });
    let Some(annotation) = matches.next() else {
        return Ok(None);
    };
    if matches.next().is_some() {
        return Err("duplicate @cors annotation".to_string());
    }
    parse_cors(annotation).map(Some)
}

fn parse_cors(annotation: &hir::Annotation) -> Result<HttpCorsProfile, String> {
    let Some(params) = annotation_params(annotation) else {
        return Ok(HttpCorsProfile::Any);
    };
    match params {
        hir::AnnotationParams::ConstExpr(expr) => {
            Ok(HttpCorsProfile::Origins(parse_const_expr_origins(expr)?))
        }
        hir::AnnotationParams::Positional(values) => Ok(HttpCorsProfile::Origins(
            values
                .iter()
                .map(parse_string_const_expr)
                .collect::<Result<Vec<_>, _>>()?,
        )),
        hir::AnnotationParams::Raw(_) | hir::AnnotationParams::Params(_) => {
            Err(cors_syntax_error())
        }
    }
}

fn parse_const_expr_origins(expr: &hir::ConstExpr) -> Result<Vec<String>, String> {
    Ok(vec![parse_string_const_expr(expr)?])
}

fn parse_string_const_expr(expr: &hir::ConstExpr) -> Result<String, String> {
    match expr {
        hir::ConstExpr::Literal(hir::Literal::StringLiteral(value)) => parse_origin_literal(value),
        _ => Err(cors_syntax_error()),
    }
}

fn parse_origin_literal(value: &str) -> Result<String, String> {
    let Some(value) = trim_string_literal(value) else {
        return Err(cors_syntax_error());
    };
    if value.is_empty() {
        return Err("@cors origins must not be empty".to_string());
    }
    if !is_valid_origin(&value) {
        return Err(format!("invalid @cors origin '{value}'"));
    }
    Ok(value)
}

fn cors_syntax_error() -> String {
    "@cors only accepts comma-separated string literals".to_string()
}

fn is_valid_origin(value: &str) -> bool {
    value == "*"
        || (value.is_ascii()
            && !value.bytes().any(|byte| byte.is_ascii_control())
            && !value.is_empty())
}

fn trim_string_literal(value: &str) -> Option<String> {
    let value = value.trim();
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        Some(value[1..value.len() - 1].to_string())
    } else {
        None
    }
}
