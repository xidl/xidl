//! Operation-level assembly (request upcasts, error downcasts, result
//! wrappers) for the rust version-adapter module.

use convert_case::{Case, Casing};

use crate::error::{IdlcError, IdlcResult};
use crate::generate::adapter::{OperationPlan, StepPlan};
use crate::generate::rust::util::rust_ident;
use crate::generate::rust_axum::interface::interface_method_support::method_struct_prefix;
use xidl_parser::hir;

use super::RenderState;
use super::assembly::StepNames;
use super::context::{
    Assignment, ConversionContext, ConversionKind, CopyStep, DecisionContext, ErrorArm,
    ErrorDowncast, FieldContext, HookMember, RequestStruct, ResultWrapper,
};
use super::naming::{error_variant, optionalized_type};

impl<'a> StepNames<'a> {
    pub(super) fn push_request_conversion(
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
        let target_ns = self.step.to.ident.as_str();
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

    pub(super) fn push_error_downcast(
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

    pub(super) fn push_result_wrapper(
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
}
