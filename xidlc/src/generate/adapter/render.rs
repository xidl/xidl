//! TypeScript rendering of the version-adapter plan.
//!
//! Rust assembles the render context — names, type expressions, decision
//! shapes — while the minijinja template renders the statements and the
//! data access they need.

use std::collections::BTreeMap;
use std::collections::HashMap;

use convert_case::{Case, Casing};
use serde::Serialize;
use xidl_parser::rest_hir::RestHirDocument;
use xidl_parser::versioning::{self, MemberInterval, VersionDef};

use crate::error::{IdlcError, IdlcResult};
use crate::generate::adapter::{
    AdapterPlan, ConversionPlan, OperationPlan, StepPlan, TypeKind, TypePlan,
};
use crate::generate::typescript::TypescriptRenderer;
use crate::generate::typescript::definition::contexts::TsType;
use crate::generate::typescript::definition::method::TypeRefTarget;
use crate::generate::typescript::definition::names::{method_struct_prefix, ts_prop_name};
use crate::generate::typescript::definition::type_expr::ts_type_for_type_spec;
use xidl_parser::hir;

/// Which imported module a versioned type reference resolves through.
#[derive(Clone, Copy)]
enum TypeRoot {
    /// Model declarations, imported as `types`.
    Models,
    /// Operation request structs, imported as `ifaceTypes`.
    Iface,
}

/// What a conversion reads and produces.
struct ConversionSource<'a> {
    /// The paired declaration: a type, or a request struct name.
    name: &'a str,
    /// Full function name of the conversion.
    fn_name: String,
    kind: TypeKind,
    upcast: bool,
    root: TypeRoot,
}

/// The full adapter module context.
#[derive(Serialize)]
struct AdapterFileContext {
    file_stem: String,
    decisions: Vec<DecisionContext>,
    hooks: Vec<HookContext>,
    error_hooks: Vec<ErrorHookContext>,
    steps: Vec<StepContext>,
}

/// One version step: conversions plus operation entry points.
#[derive(Serialize)]
struct StepContext {
    from_ident: String,
    from_pascal: String,
    from_value: String,
    to_value: String,
    up_fns: BTreeMap<String, String>,
    down_fns: BTreeMap<String, String>,
    conversions: Vec<ConversionContext>,
    operations: Vec<OperationContext>,
}

/// One conversion function: a type or a request struct, one direction.
#[derive(Serialize)]
struct ConversionContext {
    fn_name: String,
    camel: String,
    verb: &'static str,
    prep: &'static str,
    kind: ConversionKind,
    source_ty: TsType,
    target_ty: TsType,
    copies: Vec<CopyStep>,
    hook: bool,
    assign: Vec<DecisionField>,
    enum_shared: Vec<String>,
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
    prop: String,
    raw: String,
    tmp: String,
    /// Full name of the conversion to call when the member's type evolves.
    call: Option<String>,
    optional: bool,
    target_key: String,
}

/// One decision interface a hook returns.
#[derive(Serialize)]
struct DecisionContext {
    pascal: String,
    verb: &'static str,
    prep: &'static str,
    from_pascal: String,
    fields: Vec<DecisionField>,
}

#[derive(Serialize)]
struct DecisionField {
    key: String,
    raw: String,
    ty: TsType,
}

/// One member of the `VersionAdapters` interface.
#[derive(Serialize)]
struct HookContext {
    camel: String,
    pascal: String,
    verb: &'static str,
    prep: &'static str,
    from_pascal: String,
    params: Vec<HookParam>,
    /// Set when the hook returns a decision interface; unset when it
    /// returns the paired enum, typed through `ret_ty`.
    decision: bool,
    ret_ty: Option<TsType>,
}

#[derive(Serialize)]
struct HookParam {
    name: String,
    ty: TsType,
}

/// One error-mapping hook for an exception the older version cannot raise.
#[derive(Serialize)]
struct ErrorHookContext {
    camel: String,
    ex_pascal: String,
    from_pascal: String,
}

/// One operation's entry points for one step.
#[derive(Serialize)]
struct OperationContext {
    type_path: String,
    request: Option<RequestContext>,
    result: Option<ResultContext>,
}

#[derive(Serialize)]
struct RequestContext {
    entry: String,
    from_ident: String,
    target_ty: TsType,
}

#[derive(Serialize)]
struct ResultContext {
    prefix: String,
    result_type: String,
    head_ty: TsType,
    old_ty: TsType,
}

/// Generation property that turns adapter generation on; versioned
/// contracts keep rendering plain per-version output unless it is set.
pub(crate) const ENABLE_VERSION_ADAPTERS_PROPERTY: &str = "enable_version_adapters";

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
    let enabled = props
        .get(ENABLE_VERSION_ADAPTERS_PROPERTY)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let timeline = props.get(versioning::API_VERSION_TIMELINE_PROPERTY);
    if !enabled || timeline.is_none() {
        return Ok(None);
    }
    let versions: Vec<VersionDef> = serde_json::from_value(
        timeline.cloned().unwrap_or(serde_json::Value::Null),
    )
    .map_err(|error| {
        IdlcError::fmt(format!(
            "version adapters: the declared timeline is malformed: {error}"
        ))
    })?;
    let intervals: Vec<MemberInterval> = serde_json::from_value(
        props
            .get(versioning::API_VERSION_INTERVALS_PROPERTY)
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    )
    .map_err(|error| {
        IdlcError::fmt(format!(
            "version adapters: the declaration intervals are malformed: {error}"
        ))
    })?;
    let surfaces = super::surface::collect(spec, rest_hir);
    let plan = super::derive(&surfaces, &versions, &intervals)?;
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

/// Function name of a type conversion; upcasting reads the older version.
fn conversion_fn(upcast: bool, name: &str, from_ident: &str) -> String {
    let pascal = name.to_case(Case::Pascal);
    let from_pascal = from_ident.to_case(Case::Pascal);
    if upcast {
        format!("upcast{pascal}From{from_pascal}")
    } else {
        format!("downcast{pascal}To{from_pascal}")
    }
}

/// The versioned type reference a named declaration resolves to.
fn versioned_scoped(root: TypeRoot, namespace: &str, name: &str) -> TsType {
    let module = match root {
        TypeRoot::Models => "types",
        TypeRoot::Iface => "ifaceTypes",
    };
    TsType::ScopedName(format!("{module}.{namespace}.{name}"))
}

/// A member's written type in the namespace that declares it.
fn member_type(ty: &hir::TypeSpec, namespace: &str) -> IdlcResult<TsType> {
    if let hir::TypeSpec::ScopedName(scoped) = ty {
        if scoped.name.len() > 1 {
            return Err(IdlcError::fmt(
                "types nested in modules inside a version module are not supported by version adapters yet",
            ));
        }
    }
    Ok(ts_type_for_type_spec(
        ty,
        std::slice::from_ref(&namespace.to_string()),
        TypeRefTarget::Client,
    ))
}

/// The name a copy's source type resolves to when it has its own conversion.
fn nested_plan(
    ty: &Option<hir::TypeSpec>,
    fns: &BTreeMap<String, String>,
) -> IdlcResult<Option<String>> {
    let Some(spec) = ty else {
        return Ok(None);
    };
    match spec {
        hir::TypeSpec::ScopedName(scoped) => {
            let Some(last) = scoped.name.last() else {
                return Ok(None);
            };
            if fns.contains_key(last) {
                return Ok(Some(fns[last].clone()));
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

fn nested_error(name: &str) -> IdlcError {
    IdlcError::fmt(format!(
        "member type {name} nests an evolving type inside a sequence, map or template; declare it as a direct member so the adapter can project it"
    ))
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

fn verb_word(upcast: bool) -> &'static str {
    if upcast { "Upcast" } else { "Downcast" }
}

fn verb_lower(upcast: bool) -> &'static str {
    if upcast { "upcast" } else { "downcast" }
}

fn prep_word(upcast: bool) -> &'static str {
    if upcast { "From" } else { "To" }
}
