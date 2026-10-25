//! Serializable Rust rendering contexts; no cross-version inference lives here.

use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Serialize, Default)]
pub(super) struct FileContext {
    pub versions: Vec<Version>,
    pub hooks: Vec<Hook>,
    pub decisions: Vec<Decision>,
    pub requests: Vec<Decision>,
    pub conversions: Vec<Conversion>,
    pub operations: Vec<Operation>,
    pub endpoints: Vec<Endpoint>,
    pub dispatches: Vec<Dispatch>,
    pub wire: Vec<WireDefinition>,
    pub types: Vec<TypeEndpoint>,
    pub helpers: BTreeSet<&'static str>,
}

#[derive(Serialize)]
pub(super) struct Version {
    pub id: String,
    pub module: String,
    pub raw_id: String,
    pub value: String,
}

#[derive(Serialize)]
pub(super) struct Hook {
    pub name: String,
    pub source: String,
    pub target: String,
}

#[derive(Serialize)]
pub(super) struct Field {
    pub name: String,
    pub ty: String,
}

#[derive(Serialize)]
pub(super) struct Decision {
    pub name: String,
    pub fields: Vec<Field>,
}

#[derive(Serialize)]
pub(super) struct Conversion {
    pub name: String,
    pub source: String,
    pub target: String,
    pub kind: &'static str,
    pub hook: Option<String>,
    pub copies: Vec<Copy>,
    pub assignments: Vec<String>,
    pub absences: Vec<String>,
    pub variants: Vec<String>,
    pub changed: Vec<ChangedVariant>,
    pub changed_type: String,
    pub value: Option<Value>,
    pub source_shape: Shape,
    pub target_shape: Shape,
    pub version: String,
    pub upward: bool,
    pub type_path: String,
}

#[derive(Serialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub(super) enum Shape {
    Struct,
    Direct,
    Unit,
}

#[derive(Serialize)]
pub(super) struct Copy {
    pub source: String,
    pub target: String,
    pub value: Value,
}

#[derive(Serialize)]
#[serde(tag = "kind", content = "data")]
pub(super) enum Value {
    Identity,
    Named(String),
    Sequence(Box<Self>),
    Map {
        key: Box<Self>,
        value: Box<Self>,
    },
    Optional(Box<Self>),
    Array(Box<Self>),
    Boxed {
        source: bool,
        target: bool,
        value: Box<Self>,
    },
}

impl Value {
    pub(super) fn helpers(&self, used: &mut BTreeSet<&'static str>) {
        match self {
            Self::Sequence(value) => {
                used.insert("sequence");
                value.helpers(used);
            }
            Self::Array(value) => {
                used.insert("array");
                value.helpers(used);
            }
            Self::Map { key, value } => {
                used.insert("record");
                key.helpers(used);
                value.helpers(used);
            }
            Self::Optional(value) | Self::Boxed { value, .. } => value.helpers(used),
            Self::Identity | Self::Named(_) => {}
        }
    }
}

#[derive(Serialize)]
pub(super) struct ChangedVariant {
    pub name: String,
    pub ty: Option<String>,
}

#[derive(Serialize)]
pub(super) struct ErrorArm {
    pub source: String,
    pub target: Option<String>,
    pub call: Option<String>,
    pub hook: Option<String>,
}

#[derive(Serialize)]
pub(super) struct Operation {
    pub prefix: String,
    pub source_error: String,
    pub target_error: String,
    pub arms: Vec<ErrorArm>,
    pub source_typed_error: bool,
    pub target_typed_error: bool,
}

#[derive(Serialize)]
pub(super) struct TypeEndpoint {
    pub name: String,
    pub function: String,
    pub head: String,
    pub variants: Vec<TypeVariant>,
    pub exhaustive: bool,
}

#[derive(Serialize)]
pub(super) struct TypeVariant {
    pub id: String,
    pub ty: String,
    pub calls: Vec<String>,
}

#[derive(Serialize)]
pub(super) struct Endpoint {
    pub prefix: String,
    pub version_id: String,
    pub request: String,
    pub head_request: String,
    pub response: String,
    pub error: String,
    pub result: String,
    pub head_result: String,
    pub upcast: Vec<String>,
    pub downcast: Vec<String>,
    pub downcast_error: Vec<String>,
    pub parameters: Vec<WireField>,
}

#[derive(Serialize)]
pub(super) struct Dispatch {
    pub prefix: String,
    pub outcome: String,
    pub request: String,
    pub head_result: String,
    pub variants: Vec<DispatchVariant>,
    pub exhaustive: bool,
}

#[derive(Serialize)]
pub(super) struct DispatchVariant {
    pub id: String,
    pub prefix: String,
    pub result: String,
}

#[derive(Serialize)]
pub(super) struct WireField {
    pub name: String,
    pub wire: String,
    pub ty: String,
    pub optional: bool,
    pub schema: WireValue,
    pub flatten: Option<Flatten>,
}

#[derive(Serialize)]
pub(super) struct Flatten {
    pub keys: Vec<String>,
    pub receives_rest: bool,
}

#[derive(Serialize)]
pub(super) struct WireDefinition {
    pub name: String,
    pub ty: String,
    pub fields: Vec<WireField>,
    pub kind: &'static str,
    pub inner: Option<WireValue>,
    pub keys: Vec<String>,
    pub forbidden: Vec<String>,
    pub catch_all: bool,
}

#[derive(Serialize)]
#[serde(tag = "kind", content = "data")]
pub(super) enum WireValue {
    Scalar(String),
    Named(String),
    Sequence(Box<Self>),
    Map { key: String, value: Box<Self> },
    Boxed(Box<Self>),
    Optional(Box<Self>),
    Array { inner: Box<Self>, length: String },
}
