//! Data-only template contexts for TypeScript adapters.

use crate::generate::typescript::definition::contexts::TsType;
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct FileContext {
    pub file_stem: String,
    pub has_errors: bool,
    pub decisions: Vec<Decision>,
    pub hooks: Vec<Hook>,
    pub conversions: Vec<Conversion>,
    pub operations: Vec<Operation>,
    pub endpoints: Vec<Endpoint>,
    pub dispatchers: Vec<Dispatcher>,
    pub type_dispatchers: Vec<TypeDispatcher>,
    pub wire_definitions: Vec<super::wire_context::WireDefinition>,
}

#[derive(Serialize)]
pub(super) struct Field {
    pub key: String,
    pub ty: TsType,
    pub optional: bool,
}

#[derive(Serialize)]
pub(super) struct Decision {
    pub name: String,
    pub fields: Vec<Field>,
}

#[derive(Serialize)]
pub(super) struct Hook {
    pub name: String,
    pub input: TsType,
    pub output: TsType,
}

#[derive(Serialize)]
pub(super) struct Copy {
    pub source: String,
    pub target: String,
    pub value: Value,
    pub optional: bool,
}

#[derive(Serialize)]
#[serde(tag = "kind", content = "data")]
pub(super) enum Value {
    Identity,
    Named(String),
    Sequence(Box<Self>),
    Map(Box<Self>),
    Optional(Box<Self>),
}

#[derive(Serialize)]
pub(super) struct Assignment {
    pub source: String,
    pub target: String,
    pub optional: bool,
}

#[derive(Serialize)]
pub(super) struct Conversion {
    pub name: String,
    pub kind: &'static str,
    pub source: TsType,
    pub target: TsType,
    pub hook: Option<String>,
    pub copies: Vec<Copy>,
    pub assignments: Vec<Assignment>,
    pub absent_keys: Vec<String>,
    pub shared_values: Vec<String>,
    pub value: Option<Value>,
}

#[derive(Serialize, Clone)]
pub(super) struct ErrorArm {
    pub class: String,
    pub target_class: Option<String>,
    pub call: Option<String>,
    pub hook: Option<String>,
}

#[derive(Serialize, Clone)]
pub(super) struct Operation {
    pub prefix: String,
    pub from: String,
    pub version: String,
    pub type_path: String,
    pub request_source: TsType,
    pub request_target: TsType,
    pub schema: Option<String>,
    pub request_call: String,
    pub result_source: TsType,
    pub result_target: TsType,
    pub result_call: Option<String>,
    pub result_source_direct: bool,
    pub result_target_direct: bool,
    pub errors_source: Vec<String>,
    pub errors_target: Vec<String>,
    pub error_arms: Vec<ErrorArm>,
}

#[derive(Serialize)]
pub(super) struct Endpoint {
    pub prefix: String,
    pub from: String,
    pub id: String,
    pub version: String,
    pub type_path: String,
    pub schema: String,
    pub request: TsType,
    pub result: TsType,
    pub head_request: TsType,
    pub head_result: TsType,
    pub errors: Vec<String>,
    pub head_errors: Vec<String>,
    pub steps: Vec<Operation>,
}

#[derive(Serialize)]
pub(super) struct Dispatcher {
    pub prefix: String,
    pub head_request: TsType,
    pub head_result: TsType,
    pub head_errors: Vec<String>,
    pub versions: Vec<(String, String)>,
    pub results: Vec<TsType>,
    pub errors: Vec<String>,
}

#[derive(Serialize)]
pub(super) struct TypeDispatcher {
    pub name: String,
    pub head: TsType,
    pub variants: Vec<TypeVariant>,
}

#[derive(Serialize)]
pub(super) struct TypeVariant {
    pub version: String,
    pub output: TsType,
    pub calls: Vec<String>,
}
