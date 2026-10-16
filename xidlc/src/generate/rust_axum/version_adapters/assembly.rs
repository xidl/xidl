//! Type-conversion assembly for the rust version-adapter module.

use std::collections::BTreeMap;

use convert_case::{Case, Casing};

use crate::error::{IdlcError, IdlcResult};
use crate::generate::adapter::{ConversionPlan, StepPlan, TypeKind};
use crate::generate::rust::util::rust_ident;
use xidl_parser::hir;

use super::RenderState;
use super::context::{
    Assignment, ConversionContext, ConversionKind, CopyStep, DecisionContext, ErrorDowncast,
    FieldContext, HookMember, RequestStruct, ResultWrapper,
};
use super::naming::{conversion_fn, deep_plan_hit, nested_error, versioned_type};

/// Conversion function names of one step, for declarations and nested calls.
pub(super) struct StepNames<'a> {
    pub(super) step: &'a StepPlan,
    up: BTreeMap<String, String>,
    pub(super) down: BTreeMap<String, String>,
}

impl<'a> StepNames<'a> {
    pub(super) fn new(step: &'a StepPlan) -> Self {
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

    pub(super) fn push_type_conversions(
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
    pub(super) fn push_operation_conversions(
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

    pub(super) fn nested_call(
        &self,
        ty: &Option<hir::TypeSpec>,
        upcast: bool,
    ) -> IdlcResult<Option<String>> {
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
