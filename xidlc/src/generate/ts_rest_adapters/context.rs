//! Data-only template contexts for TypeScript adapters.

use crate::generate::typescript::definition::contexts::TsType;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Serialize)]
pub(super) struct FileContext {
    pub file_stem: String,
    pub has_errors: bool,
    pub has_updates: bool,
    pub decisions: Vec<Decision>,
    pub hooks: Vec<Hook>,
    pub readers: Vec<Reader>,
    pub needs: Vec<Needs>,
    pub update_stored: Vec<StoredBag>,
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

/// A hook may read only the stored values its own step removed, and an
/// update hook may also merge against the Head resource it receives.
#[derive(Serialize)]
pub(super) struct Hook {
    pub name: String,
    pub input: TsType,
    pub output: TsType,
    pub stored: Vec<String>,
    pub current: Option<TsType>,
}

/// Members one update operation's handler persists, named by its request chain.
#[derive(Serialize)]
pub(super) struct StoredBag {
    pub name: String,
    pub fields: Vec<Field>,
}

/// One stored-value accessor per removed member, keyed by the Head-side
/// instance that owns it; the application resolves identity itself.
#[derive(Serialize)]
pub(super) struct Reader {
    pub name: String,
    pub input: TsType,
    pub output: TsType,
    pub wire: String,
    pub version: String,
}

/// Wire names one served version still reads, newest version first.
#[derive(Serialize)]
pub(super) struct Needs {
    pub key: String,
    pub entries: Vec<(String, Vec<String>)>,
}

#[derive(Serialize)]
pub(super) struct Copy {
    pub source: String,
    pub target: String,
    pub value: Value,
    pub optional: bool,
}

#[derive(Serialize, Clone)]
pub(super) struct Call {
    pub name: String,
    pub stored: bool,
    pub current: bool,
    pub pair: bool,
}

#[derive(Serialize)]
#[serde(tag = "kind", content = "data")]
pub(super) enum Value {
    Identity,
    Named(Call),
    Sequence(Box<Self>),
    Map(Box<Self>),
    Optional(Box<Self>),
}

impl Value {
    /// Mark named conversions that thread stored values.
    pub(super) fn annotate(&mut self, stored: &BTreeSet<String>) {
        match self {
            Self::Named(call) => call.stored = stored.contains(&call.name),
            Self::Sequence(value) | Self::Map(value) | Self::Optional(value) => {
                value.annotate(stored);
            }
            Self::Identity => {}
        }
    }

    /// Copy the call shape of each named conversion into this value.
    pub(super) fn mark_calls(&mut self, marked: &BTreeMap<String, (bool, bool)>) {
        match self {
            Self::Named(call) => {
                if let Some((current, pair)) = marked.get(&call.name) {
                    call.current = *current;
                    call.pair = *pair;
                }
            }
            Self::Sequence(value) | Self::Map(value) | Self::Optional(value) => {
                value.mark_calls(marked);
            }
            Self::Identity => {}
        }
    }

    /// Whether this value calls any of the named conversions.
    pub(super) fn references(&self, names: &BTreeSet<String>) -> bool {
        match self {
            Self::Named(call) => names.contains(&call.name),
            Self::Sequence(value) | Self::Map(value) | Self::Optional(value) => {
                value.references(names)
            }
            Self::Identity => false,
        }
    }
}

#[derive(Serialize)]
pub(super) struct Assignment {
    pub source: String,
    pub target: String,
    pub optional: bool,
    pub omittable: bool,
}

#[derive(Serialize)]
pub(super) struct Conversion {
    pub name: String,
    pub kind: &'static str,
    /// Locates container-element refusals: the owning type or operation key.
    pub path: String,
    pub source: TsType,
    pub target: TsType,
    pub hook: Option<String>,
    /// The hook reads stored values of this step.
    pub hook_stored: bool,
    /// The function threads stored values to its hook or to a callee.
    pub stored: bool,
    /// The function receives the Head resource of its update operation.
    pub current: bool,
    pub current_type: Option<TsType>,
    /// The hook merges against the Head resource.
    pub hook_current: bool,
    /// The function returns `UpcastValue<T, S>` and accumulates the bag.
    pub pair: bool,
    pub stored_type: Option<String>,
    /// The hook decision carries the operation's stored bag.
    pub stored_field: bool,
    pub copies: Vec<Copy>,
    pub assignments: Vec<Assignment>,
    pub shared_values: Vec<String>,
    pub value: Option<Value>,
}

#[derive(Serialize, Clone)]
pub(super) struct ErrorArm {
    pub class: String,
    pub target_class: Option<String>,
    pub call: Option<Call>,
    pub hook: Option<String>,
}

#[derive(Serialize, Clone)]
pub(super) struct Operation {
    pub current: bool,
    pub prefix: String,
    pub from: String,
    pub version: String,
    pub type_path: String,
    pub request_source: TsType,
    pub request_target: TsType,
    pub schema: Option<String>,
    pub request_call: Call,
    pub result_source: TsType,
    pub result_target: TsType,
    pub result_call: Option<Call>,
    pub result_source_direct: bool,
    pub result_target_direct: bool,
    pub errors_source: Vec<String>,
    pub errors_target: Vec<String>,
    pub error_arms: Vec<ErrorArm>,
    pub stored: bool,
    pub stored_type: Option<String>,
}

#[derive(Serialize)]
pub(super) struct Endpoint {
    pub current: bool,
    pub current_type: Option<TsType>,
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
    pub stored: bool,
    pub stored_type: Option<String>,
}

#[derive(Serialize)]
pub(super) struct Dispatcher {
    pub current: bool,
    pub current_type: Option<TsType>,
    pub prefix: String,
    pub head_request: TsType,
    pub head_result: TsType,
    pub head_errors: Vec<String>,
    pub versions: Vec<DispatcherVersion>,
    pub results: Vec<TsType>,
    pub errors: Vec<String>,
    pub stored: bool,
    pub stored_type: Option<String>,
}

#[derive(Serialize)]
pub(super) struct DispatcherVersion {
    pub id: String,
    pub pascal: String,
    pub current: bool,
    pub stored: bool,
}

#[derive(Serialize)]
pub(super) struct TypeDispatcher {
    pub name: String,
    pub head: TsType,
    pub variants: Vec<TypeVariant>,
    pub stored: bool,
}

#[derive(Serialize)]
pub(super) struct TypeVariant {
    pub version: String,
    pub output: TsType,
    pub calls: Vec<Call>,
}
