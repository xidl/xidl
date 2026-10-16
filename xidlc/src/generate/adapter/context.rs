//! Serialize render contexts of the version-adapter module.

use std::collections::BTreeMap;

use serde::Serialize;

use super::TypeKind;
use crate::generate::typescript::definition::contexts::TsType;

#[derive(Clone, Copy)]
pub(crate) enum TypeRoot {
    /// Model declarations, imported as `types`.
    Models,
    /// Operation request structs, imported as `ifaceTypes`.
    Iface,
}

/// What a conversion reads and produces.
pub(crate) struct ConversionSource<'a> {
    /// The paired declaration: a type, or a request struct name.
    pub(crate) name: &'a str,
    /// Full function name of the conversion.
    pub(crate) fn_name: String,
    pub(crate) kind: TypeKind,
    pub(crate) upcast: bool,
    pub(crate) root: TypeRoot,
}

/// The full adapter module context.
#[derive(Serialize)]
pub(crate) struct AdapterFileContext {
    pub(crate) file_stem: String,
    pub(crate) decisions: Vec<DecisionContext>,
    pub(crate) hooks: Vec<HookContext>,
    pub(crate) error_hooks: Vec<ErrorHookContext>,
    pub(crate) steps: Vec<StepContext>,
}

/// One version step: conversions plus operation entry points.
#[derive(Serialize)]
pub(crate) struct StepContext {
    pub(crate) from_ident: String,
    pub(crate) from_pascal: String,
    pub(crate) from_value: String,
    pub(crate) to_value: String,
    pub(crate) up_fns: BTreeMap<String, String>,
    pub(crate) down_fns: BTreeMap<String, String>,
    pub(crate) conversions: Vec<ConversionContext>,
    pub(crate) operations: Vec<OperationContext>,
}

/// One conversion function: a type or a request struct, one direction.
#[derive(Serialize)]
pub(crate) struct ConversionContext {
    pub(crate) fn_name: String,
    pub(crate) camel: String,
    pub(crate) verb: &'static str,
    pub(crate) prep: &'static str,
    pub(crate) kind: ConversionKind,
    pub(crate) source_ty: TsType,
    pub(crate) target_ty: TsType,
    pub(crate) copies: Vec<CopyStep>,
    pub(crate) hook: bool,
    pub(crate) assign: Vec<DecisionField>,
    pub(crate) enum_shared: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ConversionKind {
    Struct,
    Enum,
}

/// One mechanical member copy inside a conversion.
#[derive(Serialize)]
pub(crate) struct CopyStep {
    pub(crate) prop: String,
    pub(crate) raw: String,
    pub(crate) tmp: String,
    /// Full name of the conversion to call when the member's type evolves.
    pub(crate) call: Option<String>,
    pub(crate) optional: bool,
    pub(crate) target_key: String,
}

/// One decision interface a hook returns.
#[derive(Serialize)]
pub(crate) struct DecisionContext {
    pub(crate) pascal: String,
    pub(crate) verb: &'static str,
    pub(crate) prep: &'static str,
    pub(crate) from_pascal: String,
    pub(crate) fields: Vec<DecisionField>,
}

#[derive(Serialize)]
pub(crate) struct DecisionField {
    pub(crate) key: String,
    pub(crate) raw: String,
    pub(crate) ty: TsType,
}

/// One member of the `VersionAdapters` interface.
#[derive(Serialize)]
pub(crate) struct HookContext {
    pub(crate) camel: String,
    pub(crate) pascal: String,
    pub(crate) verb: &'static str,
    pub(crate) prep: &'static str,
    pub(crate) from_pascal: String,
    pub(crate) params: Vec<HookParam>,
    /// Set when the hook returns a decision interface; unset when it
    /// returns the paired enum, typed through `ret_ty`.
    pub(crate) decision: bool,
    pub(crate) ret_ty: Option<TsType>,
}

#[derive(Serialize)]
pub(crate) struct HookParam {
    pub(crate) name: String,
    pub(crate) ty: TsType,
}

/// One error-mapping hook for an exception the older version cannot raise.
#[derive(Serialize)]
pub(crate) struct ErrorHookContext {
    pub(crate) camel: String,
    pub(crate) ex_pascal: String,
    pub(crate) from_pascal: String,
}

/// One operation's entry points for one step.
#[derive(Serialize)]
pub(crate) struct OperationContext {
    pub(crate) type_path: String,
    pub(crate) request: Option<RequestContext>,
    pub(crate) result: Option<ResultContext>,
}

#[derive(Serialize)]
pub(crate) struct RequestContext {
    pub(crate) entry: String,
    pub(crate) from_ident: String,
    pub(crate) target_ty: TsType,
}

#[derive(Serialize)]
pub(crate) struct ResultContext {
    pub(crate) prefix: String,
    pub(crate) result_type: String,
    pub(crate) head_ty: TsType,
    pub(crate) old_ty: TsType,
}
