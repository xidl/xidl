use super::*;
use convert_case::{Case, Casing};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};

#[cfg(test)]
mod tests;
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Annotation {
    Id {
        value: String,
    },
    Key {
        value: Option<String>,
    },
    AutoId {
        value: Option<String>,
    },
    Optional {
        value: Option<String>,
    },
    Position {
        value: String,
    },
    Value {
        value: String,
    },
    Extensibility {
        kind: String,
    },
    Final,
    Appendable,
    Mutable,
    MustUnderstand {
        value: Option<String>,
    },
    Default {
        value: String,
    },
    Range {
        min: String,
        max: String,
    },
    Min {
        value: String,
    },
    Max {
        value: String,
    },
    Unit {
        value: String,
    },
    BitBound {
        value: String,
    },
    External {
        value: Option<String>,
    },
    Nested {
        value: Option<String>,
    },
    Verbatim {
        language: Option<String>,
        placement: Option<String>,
        text: String,
    },
    Service {
        platform: Option<String>,
    },
    Oneway {
        value: Option<String>,
    },
    Ami {
        value: Option<String>,
    },
    HashId {
        value: Option<String>,
    },
    DefaultNested {
        value: Option<String>,
    },
    IgnoreLiteralNames {
        value: Option<String>,
    },
    TryConstruct {
        value: Option<String>,
    },
    NonSerialized {
        value: Option<String>,
    },
    DataRepresentation {
        kinds: Vec<String>,
    },
    Topic {
        name: Option<String>,
        platform: Option<String>,
    },
    Choice,
    Empty,
    DdsService,
    DdsRequestTopic {
        name: String,
    },
    DdsReplyTopic {
        name: String,
    },
    Builtin {
        name: String,
        params: Option<AnnotationParams>,
    },
    ScopedName {
        name: ScopedName,
        params: Option<AnnotationParams>,
    },
    DefaultLiteral,
    Rename {
        name: String,
    },
    SerializeName {
        serialize: String,
    },
    DeserializeName {
        deserialize: Vec<String>,
    },
    RenameAll {
        rule: RenameRule,
    },
    Skip,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RenameRule {
    None,
    LowerCase,
    UpperCase,
    PascalCase,
    CamelCase,
    SnakeCase,
    ScreamingSnakeCase,
    KebabCase,
    ScreamingKebabCase,
}

impl RenameRule {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::LowerCase => "lowercase",
            Self::UpperCase => "UPPERCASE",
            Self::PascalCase => "PascalCase",
            Self::CamelCase => "camelCase",
            Self::SnakeCase => "snake_case",
            Self::ScreamingSnakeCase => "SCREAMING_SNAKE_CASE",
            Self::KebabCase => "kebab-case",
            Self::ScreamingKebabCase => "SCREAMING-KEBAB-CASE",
        }
    }
}
impl FromStr for RenameRule {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "lowercase" => Ok(Self::LowerCase),
            "UPPERCASE" => Ok(Self::UpperCase),
            "PascalCase" => Ok(Self::PascalCase),
            "camelCase" => Ok(Self::CamelCase),
            "snake_case" => Ok(Self::SnakeCase),
            "SCREAMING_SNAKE_CASE" | "SCREAMINGSNAKECASE" => Ok(Self::ScreamingSnakeCase),
            "kebab-case" => Ok(Self::KebabCase),
            "SCREAMING-KEBAB-CASE" => Ok(Self::ScreamingKebabCase),
            _ => Ok(Self::None),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AnnotationParams {
    ConstExpr(ConstExpr),
    Positional(Vec<ConstExpr>),
    Params(Vec<AnnotationParam>),
    Raw(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnnotationParam {
    pub ident: String,
    pub value: Option<ConstExpr>,
}

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

pub fn serialize_name(annotations: &[Annotation]) -> Option<String> {
    annotations.iter().find_map(|a| {
        if let Annotation::SerializeName { serialize } = a {
            Some(serialize.clone())
        } else {
            None
        }
    })
}

pub fn deserialize_name(annotations: &[Annotation]) -> Option<String> {
    deserialize_names(annotations).into_iter().next()
}

pub fn deserialize_aliases(annotations: &[Annotation]) -> Vec<String> {
    let mut names = deserialize_names(annotations);
    if names.len() > 1 {
        names.drain(1..).collect()
    } else {
        Vec::new()
    }
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
    field_rename(annotations)
        .or_else(|| serialize_name(annotations))
        .or_else(|| deserialize_name(annotations))
        .unwrap_or_else(|| {
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
                "serialize_name" => {
                    if let Some(p) = &params {
                        let normalized = normalize_annotation_params(p);
                        if let Some(val) = normalized
                            .get("serialize")
                            .or_else(|| normalized.get("value"))
                        {
                            return Self::SerializeName {
                                serialize: val.clone(),
                            };
                        }
                    }
                }
                "deserialize_name" => {
                    if let Some(p) = &params {
                        let normalized = normalize_annotation_params(p);
                        if let Some(val) = normalized
                            .get("deserialize")
                            .or_else(|| normalized.get("value"))
                        {
                            return Self::DeserializeName {
                                deserialize: parse_string_list(val),
                            };
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
                Some(builtin) => super::annotation_builtin::from_builtin_annotation(builtin)
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

fn deserialize_names(annotations: &[Annotation]) -> Vec<String> {
    let mut names = Vec::new();
    for annotation in annotations {
        if let Annotation::DeserializeName { deserialize } = annotation {
            names.extend(deserialize.clone());
        }
    }
    names
}

fn parse_string_list(value: &str) -> Vec<String> {
    let value = value.trim();
    if !(value.starts_with('[') && value.ends_with(']')) {
        if value.contains(',') {
            return value
                .split(',')
                .map(|part| part.trim().trim_matches('"'))
                .filter(|part| !part.is_empty())
                .map(ToString::to_string)
                .collect();
        }
        if !value.is_empty() {
            return vec![value.trim_matches('"').to_string()];
        } else {
            return vec![];
        }
    }
    let inner = &value[1..value.len() - 1];
    inner
        .split(',')
        .map(|part| part.trim().trim_matches('"'))
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn apply_rename_rule(raw_name: &str, rule: RenameRule) -> String {
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

fn render_annotation_const_expr(expr: &ConstExpr) -> String {
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
