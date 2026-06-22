use super::*;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

mod convert;
#[cfg(test)]
mod tests;

pub use convert::{
    annotation_id_value, annotation_name, annotation_params, effective_wire_name,
    expand_annotations, field_rename, is_skipped, normalize_annotation_params, rename_all,
};
#[cfg(test)]
use convert::{apply_rename_rule, render_annotation_const_expr};
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
