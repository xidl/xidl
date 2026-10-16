//! Rust rendering of the version-adapter plan for rust-axum.
//!
//! Rust assembles names, type expressions and decision shapes; the
//! minijinja template renders the statements. The adapter module is
//! appended to the contract's generated file, so it references the
//! version modules through `{version}::` paths imported from `super`.

use std::collections::BTreeMap;

use convert_case::{Case, Casing};
use serde::Serialize;

use crate::error::{IdlcError, IdlcResult};
use crate::generate::adapter::{AdapterPlan, ConversionPlan, OperationPlan, StepPlan, TypeKind};
use crate::generate::rust::util::{rust_ident, rust_type};
use crate::generate::rust_axum::interface::interface_method_support::method_struct_prefix;
use xidl_parser::hir;

/// One conversion function: a type, an enum, a request or an error enum.
#[derive(Serialize)]
struct ConversionContext {
    fn_name: String,
    kind: ConversionKind,
    params: Vec<FieldContext>,
    source_ty: Option<String>,
    target_ty: String,
    copies: Vec<CopyStep>,
    assign: Vec<Assignment>,
    absences: Vec<String>,
    enum_shared: Vec<String>,
    hook_member: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum ConversionKind {
    Struct,
    Enum,
}

/// One mechanical member copy inside a conversion.
#[derive(Serialize)]
struct CopyStep {
    field: String,
    tmp: String,
    call: Option<String>,
    optional: bool,
    target: String,
}

/// One decision field the hook supplies, copied into the target.
#[derive(Serialize)]
struct Assignment {
    name: String,
}

/// One field of a decision struct, a request struct or a function parameter.
#[derive(Serialize, Clone)]
struct FieldContext {
    name: String,
    ty: String,
}

/// One decision struct a hook returns.
#[derive(Serialize)]
struct DecisionContext {
    name: String,
    fields: Vec<FieldContext>,
}

/// The head-shaped request struct one operation's conversion produces.
#[derive(Serialize)]
struct RequestStruct {
    name: String,
    fields: Vec<FieldContext>,
}

/// One member of the `VersionAdapters` trait.
#[derive(Serialize)]
struct HookMember {
    name: String,
    params: Vec<FieldContext>,
    ret_ty: String,
}

/// One match arm of an error-enum downcast.
#[derive(Serialize)]
struct ErrorArm {
    variant: String,
    payload_conversion: Option<String>,
    hook_member: Option<String>,
}

/// One error-enum downcast function.
#[derive(Serialize)]
struct ErrorDowncast {
    fn_name: String,
    source_ty: String,
    target_ty: String,
    arms: Vec<ErrorArm>,
}

/// One result projection wrapper for a shared operation.
#[derive(Serialize)]
struct ResultWrapper {
    fn_name: String,
    head_ty: String,
    old_ty: String,
    inner: String,
}

/// The whole adapter module context.
#[derive(Serialize)]
struct ModuleContext {
    versions: Vec<String>,
    hooks: Vec<HookMember>,
    decisions: Vec<DecisionContext>,
    requests: Vec<RequestStruct>,
    conversions: Vec<ConversionContext>,
    errors: Vec<ErrorDowncast>,
    results: Vec<ResultWrapper>,
}

/// Decision and hook names accumulated across the whole plan.
#[derive(Default)]
struct RenderState {
    hooks: Vec<HookMember>,
    decisions: Vec<DecisionContext>,
}

/// Renders the version-adapter module body for the plan.
pub(crate) fn render_module(
    plan: &AdapterPlan,
    renderer: &crate::generate::rust_axum::RustAxumRenderer,
) -> IdlcResult<String> {
    let mut state = RenderState::default();
    let mut versions = Vec::new();
    let mut requests = Vec::new();
    let mut conversions = Vec::new();
    let mut errors = Vec::new();
    let mut results = Vec::new();
    for step in &plan.steps {
        versions.push(step.from.ident.clone());
        versions.push(step.to.ident.clone());
        let names = StepNames::new(step);
        names.push_type_conversions(&mut state, &mut conversions)?;
        names.push_operation_conversions(
            step,
            &mut state,
            &mut requests,
            &mut conversions,
            &mut errors,
            &mut results,
        )?;
    }
    versions.sort();
    versions.dedup();
    let context = ModuleContext {
        versions,
        hooks: state.hooks,
        decisions: state.decisions,
        requests,
        conversions,
        errors,
        results,
    };
    renderer.render_template("version_adapters.rs.j2", &context)
}

/// Conversion function names of one step, for declarations and nested calls.
struct StepNames<'a> {
    step: &'a StepPlan,
    up: BTreeMap<String, String>,
    down: BTreeMap<String, String>,
}

impl<'a> StepNames<'a> {
    fn new(step: &'a StepPlan) -> Self {
        let mut up = BTreeMap::new();
        let mut down = BTreeMap::new();
        for type_plan in &step.types {
            if type_plan.upcast.is_some() {
                up.insert(
                    type_plan.name.clone(),
                    conversion_fn(true, &type_plan.name, &step.from.ident),
                );
            }
            if type_plan.downcast.is_some() {
                down.insert(
                    type_plan.name.clone(),
                    conversion_fn(false, &type_plan.name, &step.from.ident),
                );
            }
        }
        Self { step, up, down }
    }

    fn push_type_conversions(
        &self,
        state: &mut RenderState,
        conversions: &mut Vec<ConversionContext>,
    ) -> IdlcResult<()> {
        for type_plan in &self.step.types {
            if let Some(upcast) = &type_plan.upcast {
                conversions.push(self.render_conversion(
                    &type_plan.name,
                    type_plan.kind,
                    upcast,
                    true,
                    state,
                )?);
            }
            if let Some(downcast) = &type_plan.downcast {
                conversions.push(self.render_conversion(
                    &type_plan.name,
                    type_plan.kind,
                    downcast,
                    false,
                    state,
                )?);
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn push_operation_conversions(
        &self,
        step: &'a StepPlan,
        state: &mut RenderState,
        requests: &mut Vec<RequestStruct>,
        conversions: &mut Vec<ConversionContext>,
        errors: &mut Vec<ErrorDowncast>,
        results: &mut Vec<ResultWrapper>,
    ) -> IdlcResult<()> {
        for operation in &step.operations {
            self.push_request_conversion(step, operation, state, requests, conversions)?;
            self.push_error_downcast(step, operation, state, errors)?;
            self.push_result_wrapper(step, operation, results);
        }
        Ok(())
    }

    fn push_request_conversion(
        &self,
        step: &'a StepPlan,
        operation: &OperationPlan,
        state: &mut RenderState,
        requests: &mut Vec<RequestStruct>,
        conversions: &mut Vec<ConversionContext>,
    ) -> IdlcResult<()> {
        let Some(upcast) = &operation.upcast else {
            return Ok(());
        };
        let step_from = &step.from;
        let prefix = method_struct_prefix(&operation.interface, &operation.name);
        let from_pascal = step_from.ident.to_case(Case::Pascal);
        let struct_name = format!("{prefix}From{from_pascal}");
        let decision_name = format!("{prefix}UpcastDecisionFrom{from_pascal}");
        let hook_member = format!(
            "{}_upcast_from_{}",
            prefix.to_case(Case::Snake),
            step_from.ident
        );
        let source_ns = step_from.ident.as_str();
        let target_ns = step.to.ident.as_str();
        let mut params = Vec::new();
        for copy in &upcast.copies {
            let Some(param_ty) = &copy.from.ty else {
                return Err(IdlcError::fmt("request parameter must carry a type"));
            };
            params.push(FieldContext {
                name: rust_ident(&copy.from.ident),
                ty: optionalized_type(param_ty, source_ns, copy.from.optional)?,
            });
        }
        let mut copies = Vec::new();
        let mut assign = Vec::new();
        let mut fields = Vec::new();
        for (index, copy) in upcast.copies.iter().enumerate() {
            let target = rust_ident(&copy.to.ident);
            copies.push(CopyStep {
                field: rust_ident(&copy.from.ident),
                tmp: format!("converted{index}"),
                call: self.nested_call(&copy.from.ty, true)?,
                optional: copy.from.optional,
                target: target.clone(),
            });
            let Some(copy_ty) = &copy.to.ty else {
                return Err(IdlcError::fmt("request parameter must carry a type"));
            };
            let ty = optionalized_type(copy_ty, target_ns, copy.to.optional)?;
            fields.push(FieldContext { name: target, ty });
        }
        if let Some(hook) = &upcast.hook {
            let mut decision_fields = Vec::new();
            for field in &hook.produce {
                let ty = field
                    .member
                    .ty
                    .as_ref()
                    .ok_or_else(|| IdlcError::fmt("hook field must carry a type"))?;
                let name = rust_ident(&field.member.ident);
                if field.ignored {
                    decision_fields.push(FieldContext {
                        name,
                        ty: "Ignored".to_string(),
                    });
                } else {
                    let ty = optionalized_type(ty, target_ns, field.member.optional)?;
                    assign.push(Assignment { name: name.clone() });
                    fields.push(FieldContext {
                        name: name.clone(),
                        ty: ty.clone(),
                    });
                    decision_fields.push(FieldContext { name, ty });
                }
            }
            state.decisions.push(DecisionContext {
                name: decision_name.clone(),
                fields: decision_fields,
            });
            state.hooks.push(HookMember {
                name: hook_member.clone(),
                params: params.clone(),
                ret_ty: decision_name.clone(),
            });
        }
        requests.push(RequestStruct {
            name: struct_name.clone(),
            fields,
        });
        conversions.push(ConversionContext {
            fn_name: format!(
                "upcast_{}_from_{}",
                prefix.to_case(Case::Snake),
                step_from.ident
            ),
            kind: ConversionKind::Struct,
            params,
            source_ty: None,
            target_ty: struct_name,
            copies,
            assign,
            absences: Vec::new(),
            enum_shared: Vec::new(),
            hook_member: upcast.hook.is_some().then_some(hook_member),
        });
        Ok(())
    }

    fn push_error_downcast(
        &self,
        step: &'a StepPlan,
        operation: &OperationPlan,
        state: &mut RenderState,
        errors: &mut Vec<ErrorDowncast>,
    ) -> IdlcResult<()> {
        if operation.raises_to.is_empty() {
            return Ok(());
        }
        let prefix = method_struct_prefix(&operation.interface, &operation.name);
        let source_ty = format!("{}::{}Error", step.to.ident, prefix);
        let target_ty = format!("{}::{}Error", step.from.ident, prefix);
        let shared: std::collections::BTreeSet<&str> = operation
            .raises_from
            .iter()
            .filter(|name| operation.raises_to.contains(name))
            .map(String::as_str)
            .collect();
        let mut arms = Vec::new();
        for exception in &operation.raises_to {
            let variant = error_variant(exception)?;
            if operation.raises_added.contains(exception) {
                let hook_member = format!(
                    "{}_{}_downcast_to_{}",
                    prefix.to_case(Case::Snake),
                    exception.to_case(Case::Snake),
                    step.from.ident
                );
                state.hooks.push(HookMember {
                    name: hook_member.clone(),
                    params: vec![FieldContext {
                        name: "head".to_string(),
                        ty: format!("{}::{}", step.to.ident, rust_ident(exception)),
                    }],
                    ret_ty: target_ty.clone(),
                });
                arms.push(ErrorArm {
                    variant,
                    payload_conversion: None,
                    hook_member: Some(hook_member),
                });
            } else if shared.contains(exception.as_str()) {
                arms.push(ErrorArm {
                    variant,
                    payload_conversion: self.down.get(exception).cloned(),
                    hook_member: None,
                });
            }
        }
        arms.push(ErrorArm {
            variant: "Framework".to_string(),
            payload_conversion: None,
            hook_member: None,
        });
        errors.push(ErrorDowncast {
            fn_name: format!(
                "downcast_{}_error_to_{}",
                prefix.to_case(Case::Snake),
                step.from.ident
            ),
            source_ty,
            target_ty,
            arms,
        });
        Ok(())
    }

    fn push_result_wrapper(
        &self,
        step: &'a StepPlan,
        operation: &OperationPlan,
        results: &mut Vec<ResultWrapper>,
    ) {
        let Some(hir::TypeSpec::ScopedName(scoped)) = &operation.return_type else {
            return;
        };
        let Some(name) = scoped.name.last() else {
            return;
        };
        let Some(inner) = self.down.get(name) else {
            return;
        };
        let prefix = method_struct_prefix(&operation.interface, &operation.name);
        results.push(ResultWrapper {
            fn_name: format!(
                "downcast_{}_result_to_{}",
                prefix.to_case(Case::Snake),
                step.from.ident
            ),
            head_ty: format!("{}::{}", step.to.ident, rust_ident(name)),
            old_ty: format!("{}::{}", step.from.ident, rust_ident(name)),
            inner: inner.clone(),
        });
    }

    fn nested_call(&self, ty: &Option<hir::TypeSpec>, upcast: bool) -> IdlcResult<Option<String>> {
        let Some(spec) = ty else {
            return Ok(None);
        };
        let fns = if upcast { &self.up } else { &self.down };
        match spec {
            hir::TypeSpec::ScopedName(scoped) => {
                let Some(last) = scoped.name.last() else {
                    return Ok(None);
                };
                if let Some(name) = fns.get(last) {
                    return Ok(Some(name.clone()));
                }
                deep_plan_hit(spec, fns)
                    .map(|nested| Err(nested_error(&nested)))
                    .unwrap_or(Ok(None))
            }
            spec => deep_plan_hit(spec, fns)
                .map(|nested| Err(nested_error(&nested)))
                .unwrap_or(Ok(None)),
        }
    }

    fn render_conversion(
        &self,
        name: &str,
        kind: TypeKind,
        plan: &ConversionPlan,
        upcast: bool,
        state: &mut RenderState,
    ) -> IdlcResult<ConversionContext> {
        let step = self.step;
        let source_ns = if upcast {
            step.from.ident.as_str()
        } else {
            step.to.ident.as_str()
        };
        let target_ns = if upcast {
            step.to.ident.as_str()
        } else {
            step.from.ident.as_str()
        };
        let fns = if upcast { &self.up } else { &self.down };
        let fn_name = fns
            .get(name)
            .cloned()
            .ok_or_else(|| IdlcError::fmt("conversion must carry its declared function name"))?;
        let mut copies = Vec::new();
        for (index, copy) in plan.copies.iter().enumerate() {
            copies.push(CopyStep {
                field: rust_ident(&copy.from.ident),
                tmp: format!("converted{index}"),
                call: self.nested_call(&copy.from.ty, upcast)?,
                optional: copy.from.optional,
                target: rust_ident(&copy.to.ident),
            });
        }
        let mut assign = Vec::new();
        let mut enum_shared = Vec::new();
        let mut hook_member = None;
        if let Some(hook) = &plan.hook {
            let member_suffix = format!(
                "{}_{}_{}",
                if upcast { "upcast" } else { "downcast" },
                if upcast { "from" } else { "to" },
                step.from.ident
            );
            if kind == TypeKind::Members {
                hook_member = Some(format!("{}_{}", name.to_case(Case::Snake), member_suffix));
                let decision_name = format!(
                    "{}{}Decision{}{}",
                    name.to_case(Case::Pascal),
                    if upcast { "Upcast" } else { "Downcast" },
                    if upcast { "From" } else { "To" },
                    step.from.ident.to_case(Case::Pascal)
                );
                let mut fields = Vec::new();
                for field in &hook.produce {
                    let ty = field
                        .member
                        .ty
                        .as_ref()
                        .ok_or_else(|| IdlcError::fmt("hook field must carry a type"))?;
                    let field_name = rust_ident(&field.member.ident);
                    if field.ignored {
                        fields.push(FieldContext {
                            name: field_name,
                            ty: "Ignored".to_string(),
                        });
                    } else {
                        let ty = versioned_type(ty, target_ns)?;
                        assign.push(Assignment {
                            name: field_name.clone(),
                        });
                        fields.push(FieldContext {
                            name: field_name,
                            ty,
                        });
                    }
                }
                state.decisions.push(DecisionContext {
                    name: decision_name.clone(),
                    fields,
                });
                state.hooks.push(HookMember {
                    name: format!("{}_{}", name.to_case(Case::Snake), member_suffix),
                    params: vec![FieldContext {
                        name: "source".to_string(),
                        ty: format!("{}::{}", source_ns, rust_ident(name)),
                    }],
                    ret_ty: decision_name,
                });
            } else {
                hook_member = Some(format!("{}_{}", name.to_case(Case::Snake), member_suffix));
                state.hooks.push(HookMember {
                    name: format!("{}_{}", name.to_case(Case::Snake), member_suffix),
                    params: vec![FieldContext {
                        name: "value".to_string(),
                        ty: format!("{}::{}", source_ns, rust_ident(name)),
                    }],
                    ret_ty: format!("{}::{}", target_ns, rust_ident(name)),
                });
            }
        }
        if kind == TypeKind::Enum {
            enum_shared.extend(plan.copies.iter().map(|copy| rust_ident(&copy.from.ident)));
        }
        Ok(ConversionContext {
            fn_name,
            kind: if kind == TypeKind::Enum {
                ConversionKind::Enum
            } else {
                ConversionKind::Struct
            },
            params: Vec::new(),
            source_ty: Some(format!("{}::{}", source_ns, rust_ident(name))),
            target_ty: format!("{}::{}", target_ns, rust_ident(name)),
            copies,
            assign,
            absences: plan
                .absences
                .iter()
                .map(|member| rust_ident(&member.ident))
                .collect(),
            enum_shared,
            hook_member,
        })
    }
}

/// Function name of a type conversion; upcasting reads the older version.
fn conversion_fn(upcast: bool, name: &str, from_ident: &str) -> String {
    format!(
        "{}_{}_{}_{}",
        name.to_case(Case::Snake),
        if upcast { "upcast" } else { "downcast" },
        if upcast { "from" } else { "to" },
        from_ident
    )
}

/// The loud refusal for evolving types nested inside containers.
fn nested_error(name: &str) -> IdlcError {
    IdlcError::fmt(format!(
        "member type {name} nests an evolving type inside a sequence, map or template; declare it as a direct member so the adapter can project it"
    ))
}

/// The error enum variant of one exception, matching the interface renderer.
fn error_variant(exception: &str) -> IdlcResult<String> {
    let variant = rust_ident(exception).to_case(Case::Pascal);
    if variant == "Framework" {
        return Err(IdlcError::fmt(format!(
            "exception {exception} collides with the Framework error variant; rename it"
        )));
    }
    Ok(variant)
}

/// A member's written type, wrapped when the served shape carries it optional.
fn optionalized_type(ty: &hir::TypeSpec, namespace: &str, optional: bool) -> IdlcResult<String> {
    let inner = versioned_type(ty, namespace)?;
    if optional {
        Ok(format!("::std::option::Option<{inner}>"))
    } else {
        Ok(inner)
    }
}

/// A member's written type in the namespace that declares it.
fn versioned_type(ty: &hir::TypeSpec, namespace: &str) -> IdlcResult<String> {
    match ty {
        hir::TypeSpec::ScopedName(scoped) => {
            if scoped.name.len() > 1 {
                return Err(IdlcError::fmt(
                    "types nested in modules inside a version module are not supported by version adapters yet",
                ));
            }
            let Some(last) = scoped.name.last() else {
                return Err(IdlcError::fmt("scoped type without a name"));
            };
            Ok(format!("{}::{}", namespace, rust_ident(last)))
        }
        hir::TypeSpec::SequenceType(sequence) => Ok(format!(
            "::std::vec::Vec<{}>",
            versioned_type(&sequence.ty, namespace)?
        )),
        hir::TypeSpec::MapType(map) => Ok(format!(
            "::std::collections::BTreeMap<{}, {}>",
            versioned_type(&map.key, namespace)?,
            versioned_type(&map.value, namespace)?
        )),
        other => Ok(rust_type(other)),
    }
}

/// The first evolving type nested inside containers, if any.
fn deep_plan_hit(ty: &hir::TypeSpec, fns: &BTreeMap<String, String>) -> Option<String> {
    match ty {
        hir::TypeSpec::ScopedName(scoped) => scoped
            .name
            .last()
            .filter(|name| fns.contains_key(*name))
            .cloned(),
        hir::TypeSpec::SequenceType(sequence) => deep_plan_hit(&sequence.ty, fns),
        hir::TypeSpec::MapType(map) => deep_plan_hit(&map.value, fns),
        hir::TypeSpec::TemplateType(template) => {
            template.args.iter().find_map(|arg| deep_plan_hit(arg, fns))
        }
        _ => None,
    }
}
