//! TypeScript rendering of the version-adapter plan.
//!
//! Rust assembles the render context — names, type expressions, decision
//! shapes — while the minijinja template renders the statements and the
//! data access they need.

use std::collections::BTreeMap;

use convert_case::{Case, Casing};

use super::context::{
    AdapterFileContext, ConversionContext, ConversionKind, ConversionSource, CopyStep,
    DecisionContext, DecisionField, ErrorHookContext, HookContext, HookParam, OperationContext,
    RequestContext, ResultContext, StepContext, TypeRoot,
};
use super::naming::{
    conversion_fn, member_type, nested_plan, prep_word, verb_lower, verb_word, versioned_scoped,
};
use super::{AdapterPlan, ConversionPlan, OperationPlan, StepPlan, TypeKind, TypePlan};
use crate::error::{IdlcError, IdlcResult};
use crate::generate::typescript::TypescriptRenderer;
use crate::generate::typescript::definition::contexts::TsType;
use crate::generate::typescript::definition::names::{method_struct_prefix, ts_prop_name};
use std::collections::HashMap;
use xidl_parser::hir;
use xidl_parser::rest_hir::RestHirDocument;

/// Renders the version-adapter artifact for an opted-in versioned contract.
///
/// Returns `None` when the contract is unversioned or the property is off,
/// so existing output stays byte-identical.
pub(crate) fn adapters_artifact(
    spec: &hir::Specification,
    rest_hir: &RestHirDocument,
    props: &HashMap<String, serde_json::Value>,
    file_stem: &str,
    renderer: &TypescriptRenderer,
) -> IdlcResult<Option<String>> {
    let Some(plan) = super::entry::adapter_plan(spec, rest_hir, props)? else {
        return Ok(None);
    };
    render_adapters(&plan, file_stem, renderer).map(Some)
}

/// Decision and hook names accumulated across the whole plan.
#[derive(Default)]
struct RenderState {
    decisions: Vec<DecisionContext>,
    hooks: Vec<HookContext>,
    error_hooks: Vec<ErrorHookContext>,
}

/// Renders the version-adapter module for `file_stem`.
pub(crate) fn render_adapters(
    plan: &AdapterPlan,
    file_stem: &str,
    renderer: &TypescriptRenderer,
) -> IdlcResult<String> {
    let mut state = RenderState::default();
    let mut steps = Vec::new();
    for step in &plan.steps {
        steps.push(render_step(step, &mut state)?);
    }
    let context = AdapterFileContext {
        file_stem: file_stem.to_string(),
        decisions: state.decisions,
        hooks: state.hooks,
        error_hooks: state.error_hooks,
        steps,
    };
    renderer.render_template("http/version_adapters.ts.j2", &context)
}

fn render_step(step: &StepPlan, state: &mut RenderState) -> IdlcResult<StepContext> {
    let names = StepNames::new(step);
    let mut conversions = Vec::new();
    for type_plan in &step.types {
        names.push_type_conversions(type_plan, state, &mut conversions)?;
    }
    let mut operations = Vec::new();
    for operation in &step.operations {
        operations.push(names.render_operation(operation, state, &mut conversions)?);
    }
    Ok(StepContext {
        from_ident: step.from.ident.clone(),
        from_pascal: step.from.ident.to_case(Case::Pascal),
        from_value: step.from.value.clone(),
        to_value: step.to.value.clone(),
        up_fns: names.up,
        down_fns: names.down,
        conversions,
        operations,
    })
}

/// Conversion function names of one step, for declarations and nested copies.
struct StepNames<'a> {
    step: &'a StepPlan,
    from_pascal: String,
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
                    self::conversion_fn(true, &type_plan.name, &step.from.ident),
                );
            }
            if type_plan.downcast.is_some() {
                down.insert(
                    type_plan.name.clone(),
                    self::conversion_fn(false, &type_plan.name, &step.from.ident),
                );
            }
        }
        Self {
            step,
            from_pascal: step.from.ident.to_case(Case::Pascal),
            up,
            down,
        }
    }

    fn push_type_conversions(
        &self,
        type_plan: &TypePlan,
        state: &mut RenderState,
        conversions: &mut Vec<ConversionContext>,
    ) -> IdlcResult<()> {
        if let Some(upcast) = &type_plan.upcast {
            conversions.push(self.render_conversion(
                &ConversionSource {
                    name: &type_plan.name,
                    fn_name: self.up[type_plan.name.as_str()].clone(),
                    kind: type_plan.kind,
                    upcast: true,
                    root: TypeRoot::Models,
                },
                upcast,
                state,
            )?);
        }
        if let Some(downcast) = &type_plan.downcast {
            conversions.push(self.render_conversion(
                &ConversionSource {
                    name: &type_plan.name,
                    fn_name: self.down[type_plan.name.as_str()].clone(),
                    kind: type_plan.kind,
                    upcast: false,
                    root: TypeRoot::Models,
                },
                downcast,
                state,
            )?);
        }
        Ok(())
    }

    fn render_operation(
        &self,
        operation: &OperationPlan,
        state: &mut RenderState,
        conversions: &mut Vec<ConversionContext>,
    ) -> IdlcResult<OperationContext> {
        let step = self.step;
        let prefix = method_struct_prefix(&operation.interface, &operation.name);
        let request = operation
            .upcast
            .as_ref()
            .map(|plan| -> IdlcResult<RequestContext> {
                let name = format!("{prefix}Request");
                conversions.push(self.render_conversion(
                    &ConversionSource {
                        name: &name,
                        fn_name: self::conversion_fn(true, &name, &step.from.ident),
                        kind: TypeKind::Members,
                        upcast: true,
                        root: TypeRoot::Iface,
                    },
                    plan,
                    state,
                )?);
                Ok(RequestContext {
                    entry: prefix.clone(),
                    from_ident: step.from.ident.clone(),
                    target_ty: versioned_scoped(TypeRoot::Iface, &step.to.ident, &name),
                })
            })
            .transpose()?;
        let result = self.result_context(operation, &prefix)?;
        self.push_error_hooks(operation, state)?;
        Ok(OperationContext {
            type_path: format!("{}::{}", operation.interface, operation.name),
            request,
            result,
        })
    }

    fn result_context(
        &self,
        operation: &OperationPlan,
        prefix: &str,
    ) -> IdlcResult<Option<ResultContext>> {
        let step = self.step;
        let Some(hir::TypeSpec::ScopedName(scoped)) = &operation.return_type else {
            return Ok(None);
        };
        let Some(name) = scoped.name.last() else {
            return Ok(None);
        };
        if !self.down.contains_key(name) {
            return Ok(None);
        }
        Ok(Some(ResultContext {
            prefix: prefix.to_string(),
            result_type: name.clone(),
            head_ty: versioned_scoped(TypeRoot::Models, &step.to.ident, name),
            old_ty: versioned_scoped(TypeRoot::Models, &step.from.ident, name),
        }))
    }

    fn push_error_hooks(
        &self,
        operation: &OperationPlan,
        state: &mut RenderState,
    ) -> IdlcResult<()> {
        if operation.raises_added.is_empty() {
            return Ok(());
        }
        let prefix = method_struct_prefix(&operation.interface, &operation.name);
        for exception in &operation.raises_added {
            let ex_pascal = exception.to_case(Case::Pascal);
            let camel = format!("{prefix}{ex_pascal}").to_case(Case::Camel);
            state.error_hooks.push(ErrorHookContext {
                camel,
                ex_pascal,
                from_pascal: self.from_pascal.clone(),
            });
        }
        Ok(())
    }

    fn render_conversion(
        &self,
        source: &ConversionSource<'_>,
        plan: &ConversionPlan,
        state: &mut RenderState,
    ) -> IdlcResult<ConversionContext> {
        let step = self.step;
        let upcast = source.upcast;
        let pascal = source.name.to_case(Case::Pascal);
        let camel = pascal.to_case(Case::Camel);
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
        let mut copies = Vec::new();
        for (index, copy) in plan.copies.iter().enumerate() {
            copies.push(CopyStep {
                prop: ts_prop_name(&copy.from.ident),
                raw: copy.from.ident.clone(),
                tmp: format!("converted{index}"),
                call: nested_plan(&copy.from.ty, fns)?,
                optional: copy.from.optional,
                target_key: ts_prop_name(&copy.to.ident),
            });
        }
        let mut assign = Vec::new();
        let mut enum_shared = Vec::new();
        if let Some(hook_plan) = &plan.hook {
            if source.kind == TypeKind::Members {
                let mut fields = Vec::new();
                for field in &hook_plan.produce {
                    let ty = field
                        .member
                        .ty
                        .as_ref()
                        .ok_or_else(|| IdlcError::fmt("hook field must carry a type"))?;
                    let key = ts_prop_name(&field.member.ident);
                    if field.ignored {
                        fields.push(DecisionField {
                            key,
                            raw: field.member.ident.clone(),
                            ty: TsType::Primitive("Ignored".to_string()),
                        });
                    } else {
                        let typed = DecisionField {
                            key: key.clone(),
                            raw: field.member.ident.clone(),
                            ty: member_type(ty, target_ns)?,
                        };
                        fields.push(DecisionField {
                            key: typed.key.clone(),
                            raw: typed.raw.clone(),
                            ty: typed.ty.clone(),
                        });
                        assign.push(typed);
                    }
                }
                state.decisions.push(DecisionContext {
                    pascal: pascal.clone(),
                    verb: verb_word(upcast),
                    prep: prep_word(upcast),
                    from_pascal: self.from_pascal.clone(),
                    fields,
                });
                state.hooks.push(HookContext {
                    camel: camel.clone(),
                    pascal,
                    verb: verb_word(upcast),
                    prep: prep_word(upcast),
                    from_pascal: self.from_pascal.clone(),
                    params: vec![HookParam {
                        name: "source".to_string(),
                        ty: versioned_scoped(source.root, source_ns, source.name),
                    }],
                    decision: true,
                    ret_ty: None,
                });
            } else {
                state.hooks.push(HookContext {
                    camel: camel.clone(),
                    pascal: pascal.clone(),
                    verb: verb_word(upcast),
                    prep: prep_word(upcast),
                    from_pascal: self.from_pascal.clone(),
                    params: vec![HookParam {
                        name: "value".to_string(),
                        ty: TsType::InlineEnum(
                            hook_plan
                                .consumed
                                .iter()
                                .map(|member| member.wire_name.clone())
                                .collect(),
                        ),
                    }],
                    decision: false,
                    ret_ty: Some(versioned_scoped(source.root, target_ns, source.name)),
                });
            }
        }
        if source.kind == TypeKind::Enum {
            enum_shared.extend(plan.copies.iter().map(|copy| copy.from.wire_name.clone()));
        }
        Ok(ConversionContext {
            fn_name: source.fn_name.clone(),
            camel: camel.clone(),
            verb: verb_lower(upcast),
            prep: prep_word(upcast),
            kind: if source.kind == TypeKind::Enum {
                ConversionKind::Enum
            } else {
                ConversionKind::Struct
            },
            source_ty: versioned_scoped(source.root, source_ns, source.name),
            target_ty: versioned_scoped(source.root, target_ns, source.name),
            copies,
            hook: plan.hook.is_some(),
            assign,
            enum_shared,
        })
    }
}
