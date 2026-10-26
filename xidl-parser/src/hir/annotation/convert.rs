use super::super::*;
use super::params::{parse_raw_annotation_params, trim_annotation_quotes};
use super::{Annotation, AnnotationParam, AnnotationParams, RenameRule};
use crate::lint::{AnnotationDefinition, AnnotationKind};
use convert_case::{Case, Casing};
use std::collections::HashMap;

pub fn annotation_name(annotation: &Annotation) -> Option<&str> {
    let name = match annotation {
        Annotation::Builtin { name, .. } => Some(name.as_str()),
        Annotation::ScopedName { name, .. } => name.name.last().map(|value| value.as_str()),
        _ => None,
    }?;
    Some(AnnotationDefinition::lookup(name).map_or(name, |entry| entry.name))
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
    fn from(mut value: crate::typed_ast::AnnotationAppl) -> Self {
        match &mut value.name {
            crate::typed_ast::AnnotationName::Builtin(name) => {
                if let Some(definition) = AnnotationDefinition::lookup(name) {
                    *name = definition.name.to_string();
                }
            }
            crate::typed_ast::AnnotationName::ScopedName(name)
                if name.scoped_name.is_none() && !name.node_text.starts_with("::") =>
            {
                if let Some(definition) = AnnotationDefinition::lookup(&name.identifier.0) {
                    name.identifier.0 = definition.name.to_string();
                }
            }
            _ => {}
        }
        let params = value.params.map(Into::into);

        let name_ref = match &value.name {
            crate::typed_ast::AnnotationName::ScopedName(name) => Some(name.identifier.0.as_str()),
            crate::typed_ast::AnnotationName::Builtin(name) => Some(name.as_str()),
        };

        if let Some(name) = name_ref {
            match AnnotationKind::from_name(name) {
                Some(AnnotationKind::Rename | AnnotationKind::Name) => {
                    if let Some(p) = &params {
                        let normalized = normalize_annotation_params(p);
                        if let Some(val) =
                            normalized.get("value").or_else(|| normalized.get("name"))
                        {
                            return Self::Rename { name: val.clone() };
                        }
                    }
                }
                Some(AnnotationKind::RenameAll) => {
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
                Some(AnnotationKind::Skip) => return Self::Skip,
                Some(AnnotationKind::Version) => {
                    if params.is_none() {
                        return Self::Version;
                    }
                    if let Some(parameter) = params
                        .as_ref()
                        .and_then(|params| super::VersionParameter::parse(params).ok())
                    {
                        return Self::VersionParameter(parameter);
                    }
                }
                Some(AnnotationKind::Added | AnnotationKind::Removed) => {
                    if let Some(AnnotationParams::ConstExpr(ConstExpr::ScopedName(version))) =
                        params.as_ref()
                    {
                        let version = version.clone();
                        return match AnnotationKind::from_name(name) {
                            Some(AnnotationKind::Added) => Self::Added { version },
                            _ => Self::Removed { version },
                        };
                    }
                }
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

pub(crate) fn render_annotation_const_expr(expr: &ConstExpr) -> String {
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
