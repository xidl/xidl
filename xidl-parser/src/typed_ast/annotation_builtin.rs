use super::*;
use serde::{Deserialize, Serialize};

mod parse;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuiltinAnnotation {
    Id {
        value: IntegerLiteral,
    },
    AutoId {
        value: Option<AutoIdKind>,
    },
    Optional {
        value: Option<PositiveIntConst>,
    },
    Position {
        value: PositiveIntConst,
    },
    Value {
        value: ConstExpr,
    },
    Extensibility {
        kind: ExtensibilityKind,
    },
    Final,
    Appendable,
    Mutable,
    Key {
        value: Option<PositiveIntConst>,
    },
    MustUnderstand {
        value: Option<PositiveIntConst>,
    },
    DefaultLiteral,
    Default {
        value: ConstExpr,
    },
    Range {
        min: PositiveIntConst,
        max: PositiveIntConst,
    },
    Min {
        value: PositiveIntConst,
    },
    Max {
        value: PositiveIntConst,
    },
    Unit {
        value: ConstExpr,
    },
    BitBound {
        value: PositiveIntConst,
    },
    External {
        value: Option<PositiveIntConst>,
    },
    Nested {
        value: Option<PositiveIntConst>,
    },
    Verbatim {
        language: Option<VerbatimLanguage>,
        placement: Option<PlacementKind>,
        text: ConstExpr,
    },
    Service {
        platform: Option<ServicePlatform>,
    },
    Oneway {
        value: Option<ConstExpr>,
    },
    Ami {
        value: Option<ConstExpr>,
    },
    HashId {
        value: Option<ConstExpr>,
    },
    DefaultNested {
        value: Option<ConstExpr>,
    },
    IgnoreLiteralNames {
        value: Option<ConstExpr>,
    },
    TryConstruct {
        value: Option<TryConstructFailAction>,
    },
    NonSerialized {
        value: Option<BooleanLiteral>,
    },
    DataRepresentation {
        kinds: Vec<DataRepresentationKind>,
    },
    Topic {
        name: Option<ConstExpr>,
        platform: Option<TopicPlatform>,
    },
    Choice,
    Empty,
    DdsService,
    DdsRequestTopic {
        name: ConstExpr,
    },
    DdsReplyTopic {
        name: ConstExpr,
    },
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutoIdKind {
    Sequential,
    Hash,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExtensibilityKind {
    Final,
    Appendable,
    Mutable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerbatimLanguage {
    C,
    Cpp,
    Java,
    Idl,
    Any,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlacementKind {
    BeginFile,
    BeforeDeclaration,
    BeginDeclaration,
    EndDeclaration,
    AfterDeclaration,
    EndFile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServicePlatform {
    Corba,
    Dds,
    Any,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TryConstructFailAction {
    Discard,
    UseDefault,
    Trim,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataRepresentationKind {
    Xml,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TopicPlatform {
    Dds,
    Any,
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

impl AutoIdKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Sequential => "SEQUENTIAL",
            Self::Hash => "HASH",
        }
    }
}

impl ExtensibilityKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Final => "FINAL",
            Self::Appendable => "APPENDABLE",
            Self::Mutable => "MUTABLE",
        }
    }
}

impl VerbatimLanguage {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::C => "c",
            Self::Cpp => "c++",
            Self::Java => "java",
            Self::Idl => "idl",
            Self::Any => "*",
        }
    }
}

impl PlacementKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BeginFile => "BEGIN_FILE",
            Self::BeforeDeclaration => "BEFORE_DECLARATION",
            Self::BeginDeclaration => "BEGIN_DECLARATION",
            Self::EndDeclaration => "END_DECLARATION",
            Self::AfterDeclaration => "AFTER_DECLARATION",
            Self::EndFile => "END_FILE",
        }
    }
}

impl ServicePlatform {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Corba => "CORBA",
            Self::Dds => "DDS",
            Self::Any => "*",
        }
    }
}

impl TryConstructFailAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Discard => "DISCARD",
            Self::UseDefault => "USE_DEFAULT",
            Self::Trim => "TRIM",
        }
    }
}

impl DataRepresentationKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Xml => "XML",
        }
    }
}

impl TopicPlatform {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Dds => "DDS",
            Self::Any => "*",
        }
    }
}

pub(crate) use parse::parse_builtin_annotation;
