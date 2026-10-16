//! Serialize render contexts of the rust version-adapter module.

use serde::Serialize;

/// One conversion function: a type, an enum, a request or an error enum.
#[derive(Serialize)]
pub(crate) struct ConversionContext {
    pub(crate) fn_name: String,
    pub(crate) kind: ConversionKind,
    pub(crate) params: Vec<FieldContext>,
    pub(crate) source_ty: Option<String>,
    pub(crate) target_ty: String,
    pub(crate) copies: Vec<CopyStep>,
    pub(crate) assign: Vec<Assignment>,
    pub(crate) absences: Vec<String>,
    pub(crate) enum_shared: Vec<String>,
    pub(crate) hook_member: Option<String>,
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
    pub(crate) field: String,
    pub(crate) tmp: String,
    pub(crate) call: Option<String>,
    pub(crate) optional: bool,
    pub(crate) target: String,
}

/// One decision field the hook supplies, copied into the target.
#[derive(Serialize)]
pub(crate) struct Assignment {
    pub(crate) name: String,
}

/// One field of a decision struct, a request struct or a function parameter.
#[derive(Serialize, Clone)]
pub(crate) struct FieldContext {
    pub(crate) name: String,
    pub(crate) ty: String,
}

/// One decision struct a hook returns.
#[derive(Serialize)]
pub(crate) struct DecisionContext {
    pub(crate) name: String,
    pub(crate) fields: Vec<FieldContext>,
}

/// The head-shaped request struct one operation's conversion produces.
#[derive(Serialize)]
pub(crate) struct RequestStruct {
    pub(crate) name: String,
    pub(crate) fields: Vec<FieldContext>,
}

/// One member of the `VersionAdapters` trait.
#[derive(Serialize)]
pub(crate) struct HookMember {
    pub(crate) name: String,
    pub(crate) params: Vec<FieldContext>,
    pub(crate) ret_ty: String,
}

/// One match arm of an error-enum downcast.
#[derive(Serialize)]
pub(crate) struct ErrorArm {
    pub(crate) variant: String,
    pub(crate) payload_conversion: Option<String>,
    pub(crate) hook_member: Option<String>,
}

/// One error-enum downcast function.
#[derive(Serialize)]
pub(crate) struct ErrorDowncast {
    pub(crate) fn_name: String,
    pub(crate) source_ty: String,
    pub(crate) target_ty: String,
    pub(crate) arms: Vec<ErrorArm>,
}

/// One result projection wrapper for a shared operation.
#[derive(Serialize)]
pub(crate) struct ResultWrapper {
    pub(crate) fn_name: String,
    pub(crate) head_ty: String,
    pub(crate) old_ty: String,
    pub(crate) inner: String,
}

/// The whole adapter module context.
#[derive(Serialize)]
pub(crate) struct ModuleContext {
    pub(crate) versions: Vec<String>,
    pub(crate) hooks: Vec<HookMember>,
    pub(crate) decisions: Vec<DecisionContext>,
    pub(crate) requests: Vec<RequestStruct>,
    pub(crate) conversions: Vec<ConversionContext>,
    pub(crate) errors: Vec<ErrorDowncast>,
    pub(crate) results: Vec<ResultWrapper>,
}
