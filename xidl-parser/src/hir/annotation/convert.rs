use super::super::*;
use super::{Annotation, AnnotationParam, AnnotationParams, RenameRule};
use convert_case::{Case, Casing};
use std::collections::HashMap;

pub fn annotation_name(annotation: &Annotation) -> Option<&str> {
    match annotation {
        Annotation::Builtin { name, .. } => Some(name.as_str()),
        Annotation::ScopedName { name, .. } => name.name.last().map(|value| value.as_str()),
        _ => None,
    }
}

pub fn annotation_params(annotation: &Annotation) -> Option<&AnnotationParams> {
    match annotation {
        Annotation::Builtin { params, .. } => params.as_ref(),
        Annotation::ScopedName { params, .. } => params.as_ref(),
        _ => None,
    }
}

pub fn normalize_annotation_params(params: &AnnotationParams) -> HashMap<String, String> {
    let mut out = HashMap::new();
    match params {
        AnnotationParams::Raw(value) => {
            let parsed = parse_raw_annotation_params(value);
            if parsed.is_empty() {
                out.insert(
                    "value".to_string(),
                    trim_annotation_quotes(value).unwrap_or_else(|| value.clone()),
                );
            }
            for (key, value) in parsed {
                out.insert(key.to_ascii_lowercase(), value);
            }
        }
        AnnotationParams::Params(values) => {
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
        AnnotationParams::ConstExpr(expr) => {
            let rendered = render_annotation_const_expr(expr);
            out.insert(
                "value".to_string(),
                trim_annotation_quotes(&rendered).unwrap_or(rendered),
            );
        }
        AnnotationParams::Positional(values) => {
            let rendered = values
                .iter()
                .map(render_annotation_const_expr)
                .collect::<Vec<_>>()
                .join(", ");
            out.insert(
                "value".to_string(),
                trim_annotation_quotes(&rendered).unwrap_or(rendered),
            );
        }
    }
    out
}

pub fn is_skipped(annotations: &[Annotation]) -> bool {
    annotations.iter().any(|a| matches!(a, Annotation::Skip))
}

pub fn field_rename(annotations: &[Annotation]) -> Option<String> {
    annotations.iter().find_map(|a| {
        if let Annotation::Rename { name } = a {
            Some(name.clone())
        } else {
            None
        }
    })
}

pub fn rename_all(annotations: &[Annotation]) -> Option<RenameRule> {
    annotations.iter().find_map(|a| {
        if let Annotation::RenameAll { rule } = a {
            Some(rule.clone())
        } else {
            None
        }
    })
}

pub fn effective_wire_name(
    raw_name: &str,
    annotations: &[Annotation],
    container_annotations: &[Annotation],
) -> String {
    field_rename(annotations).unwrap_or_else(|| {
        if let Some(rule) = rename_all(container_annotations) {
            apply_rename_rule(raw_name, rule)
        } else {
            raw_name.to_string()
        }
    })
}

pub fn annotation_id_value(annotations: &[Annotation]) -> Option<u32> {
    for annotation in annotations {
        if let Annotation::Id { value } = annotation {
            if let Ok(value) = value.parse::<u32>() {
                return Some(value);
            }
        }
    }
    None
}

pub fn expand_annotations(values: Vec<crate::typed_ast::AnnotationAppl>) -> Vec<Annotation> {
    let mut out = Vec::new();
    for value in values {
        push_annotation(&mut out, value);
    }
    out
}

fn push_annotation(out: &mut Vec<Annotation>, mut value: crate::typed_ast::AnnotationAppl) {
    let extra = std::mem::take(&mut value.extra);
    out.push(Annotation::from(value));
    for item in extra {
        push_annotation(out, item);
    }
}

impl From<crate::typed_ast::AnnotationAppl> for Annotation {
    fn from(value: crate::typed_ast::AnnotationAppl) -> Self {
        let params = value.params.map(Into::into);

        let name_ref = match &value.name {
            crate::typed_ast::AnnotationName::ScopedName(name) => Some(name.identifier.0.as_str()),
            crate::typed_ast::AnnotationName::Builtin(name) => Some(name.as_str()),
        };

        if let Some(name) = name_ref {
            match name.to_ascii_lowercase().as_str() {
                "rename" | "name" => {
                    if let Some(p) = &params {
                        let normalized = normalize_annotation_params(p);
                        if let Some(val) =
                            normalized.get("value").or_else(|| normalized.get("name"))
                        {
                            return Self::Rename { name: val.clone() };
                        }
                    }
                }
                "rename_all" => {
                    if let Some(p) = &params {
                        let normalized = normalize_annotation_params(p);
                        if let Some(val) =
                            normalized.get("rule").or_else(|| normalized.get("value"))
                        {
                            return Self::RenameAll {
                                rule: val.parse().unwrap_or(RenameRule::None),
                            };
                        }
                    }
                }
                "skip" => return Self::Skip,
                _ => {}
            }
        }

        match value.name {
            crate::typed_ast::AnnotationName::ScopedName(name) => Self::ScopedName {
                name: name.into(),
                params,
            },
            crate::typed_ast::AnnotationName::Builtin(name) => match value.builtin {
                Some(builtin) => super::super::annotation_builtin::from_builtin_annotation(builtin)
                    .unwrap_or(Self::Builtin { name, params }),
                None => Self::Builtin { name, params },
            },
        }
    }
}

impl From<crate::typed_ast::AnnotationParams> for AnnotationParams {
    fn from(value: crate::typed_ast::AnnotationParams) -> Self {
        match value {
            crate::typed_ast::AnnotationParams::Params(params) => {
                let mut positional = Vec::new();
                let mut named = Vec::new();
                for param in params {
                    match param {
                        crate::typed_ast::AnnotationApplParam::Positional(expr) => {
                            positional.push(expr.into());
                        }
                        crate::typed_ast::AnnotationApplParam::Named { ident, value } => {
                            named.push(AnnotationParam {
                                ident: ident.0,
                                value: Some(value.into()),
                            });
                        }
                    }
                }
                if !positional.is_empty() && named.is_empty() {
                    if positional.len() == 1 {
                        Self::ConstExpr(positional.remove(0))
                    } else {
                        Self::Positional(positional)
                    }
                } else {
                    Self::Params(named)
                }
            }
            crate::typed_ast::AnnotationParams::Raw(value) => Self::Raw(value),
        }
    }
}

pub(super) fn apply_rename_rule(raw_name: &str, rule: RenameRule) -> String {
    match rule {
        RenameRule::None => raw_name.to_string(),
        RenameRule::LowerCase => raw_name.to_case(Case::Flat),
        RenameRule::UpperCase => raw_name.to_case(Case::UpperFlat),
        RenameRule::PascalCase => raw_name.to_case(Case::Pascal),
        RenameRule::CamelCase => raw_name.to_case(Case::Camel),
        RenameRule::SnakeCase => raw_name.to_case(Case::Snake),
        RenameRule::ScreamingSnakeCase => raw_name.to_case(Case::UpperSnake),
        RenameRule::KebabCase => raw_name.to_case(Case::Kebab),
        RenameRule::ScreamingKebabCase => raw_name.to_case(Case::Cobol),
    }
}

fn parse_raw_annotation_params(raw: &str) -> Vec<(String, String)> {
    let mut parts = Vec::new();
    let mut buf = String::new();
    let mut quote = None;
    let mut escaped = false;

    for ch in raw.chars() {
        if escaped {
            buf.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' && quote.is_some() {
            escaped = true;
            buf.push(ch);
            continue;
        }
        match ch {
            '\'' | '"' => {
                if quote == Some(ch) {
                    quote = None;
                } else if quote.is_none() {
                    quote = Some(ch);
                }
                buf.push(ch);
            }
            ',' if quote.is_none() => {
                let item = buf.trim();
                if !item.is_empty() {
                    parts.push(item.to_string());
                }
                buf.clear();
            }
            _ => buf.push(ch),
        }
    }

    let item = buf.trim();
    if !item.is_empty() {
        parts.push(item.to_string());
    }

    parts
        .into_iter()
        .map(|part| {
            if let Some((key, value)) = part.split_once('=') {
                let value = trim_annotation_quotes(value.trim())
                    .unwrap_or_else(|| value.trim().to_string());
                (key.trim().to_string(), unescape_param_value(&value))
            } else {
                let value =
                    trim_annotation_quotes(part.trim()).unwrap_or_else(|| part.trim().to_string());
                ("value".to_string(), unescape_param_value(&value))
            }
        })
        .collect()
}

fn unescape_param_value(value: &str) -> String {
    let mut out = String::new();
    let mut escaped = false;
    for ch in value.chars() {
        if escaped {
            out.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        out.push(ch);
    }
    out
}

fn trim_annotation_quotes(value: &str) -> Option<String> {
    let value = value.trim();
    if value.len() < 2 {
        return None;
    }
    let first = value.chars().next().unwrap();
    let last = value.chars().last().unwrap();
    if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
        Some(value[1..value.len() - 1].to_string())
    } else {
        None
    }
}

pub(super) fn render_annotation_const_expr(expr: &ConstExpr) -> String {
    match expr {
        ConstExpr::ScopedName(value) => {
            let prefix = if value.is_root { "::" } else { "" };
            format!("{prefix}{}", value.name.join("::"))
        }
        ConstExpr::Literal(value) => render_annotation_literal(value),
        ConstExpr::UnaryExpr(op, value) => {
            let op = match op {
                UnaryOperator::Add => "+",
                UnaryOperator::Sub => "-",
                UnaryOperator::Not => "~",
            };
            format!("({op}{})", render_annotation_const_expr(value))
        }
        ConstExpr::BinaryExpr(op, left, right) => {
            let op = match op {
                BinaryOperator::Or => "|",
                BinaryOperator::Xor => "^",
                BinaryOperator::And => "&",
                BinaryOperator::LeftShift => "<<",
                BinaryOperator::RightShift => ">>",
                BinaryOperator::Add => "+",
                BinaryOperator::Sub => "-",
                BinaryOperator::Mult => "*",
                BinaryOperator::Div => "/",
                BinaryOperator::Mod => "%",
            };
            format!(
                "({} {op} {})",
                render_annotation_const_expr(left),
                render_annotation_const_expr(right)
            )
        }
    }
}

fn render_annotation_literal(value: &Literal) -> String {
    match value {
        Literal::IntegerLiteral(IntegerLiteral(value)) => value.clone(),
        Literal::FloatingPtLiteral(value) => {
            let sign = value.sign.as_ref().map(IntegerSign::as_str).unwrap_or("");
            format!("{}{}.{}", sign, value.integer.0, value.fraction.0)
        }
        Literal::CharLiteral(value)
        | Literal::WideCharacterLiteral(value)
        | Literal::StringLiteral(value)
        | Literal::WideStringLiteral(value) => value.clone(),
        Literal::BooleanLiteral(value) => value.to_string(),
    }
}
