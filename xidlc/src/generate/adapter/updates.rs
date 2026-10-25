//! Select update operations and derive the `current` / `stored` upcast flow.

use super::ValuePlan;
use super::model::*;
use crate::error::{IdlcError, IdlcResult};
use crate::generate::utils::scope::resolve_canonical;
use std::collections::{BTreeMap, BTreeSet};
use xidl_parser::hir;
use xidl_parser::rest_hir::HttpMethod;

/// Per step, which upcast conversions accept `current` and which members the
/// operation envelope or a type conversion persists. The operation envelope
/// lives under its operation key; type conversions under their type key.
#[derive(Debug, Clone, Default)]
pub(crate) struct StepUpdates {
    pub current: BTreeSet<String>,
    pub paired: BTreeMap<String, Vec<Member>>,
    /// The operation whose stored bag each current conversion serves.
    pub owner: BTreeMap<String, String>,
}

/// One selected operation: the Head return type that becomes `current`, and
/// the members its handler persists across the whole timeline.
#[derive(Debug, Clone)]
pub(crate) struct OperationUpdate {
    pub return_key: String,
    pub stored: Vec<Member>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Updates {
    pub steps: Vec<StepUpdates>,
    pub operations: BTreeMap<String, OperationUpdate>,
}

impl Updates {
    /// Record members once per wire name, preserving the first shape.
    pub(crate) fn insert(entry: &mut Vec<Member>, members: &[Member]) {
        for member in members {
            if entry.iter().any(|existing| existing.wire == member.wire) {
                continue;
            }
            entry.push(member.clone());
        }
    }
}

/// Upcast conversions reachable from one operation envelope.
fn chain(step: &StepPlan, envelope: &ConversionPlan) -> BTreeSet<String> {
    let mut pending = named(envelope.copies.iter().map(|copy| &copy.value));
    let mut chain = BTreeSet::new();
    while let Some(key) = pending.pop() {
        if !chain.insert(key.clone()) {
            continue;
        }
        let upcast = step
            .types
            .iter()
            .find(|ty| ty.key == key)
            .and_then(|ty| ty.upcast.as_ref());
        let values = match upcast {
            Some(TypeConversion::Alias(value)) => vec![value],
            Some(TypeConversion::Struct(conversion))
            | Some(TypeConversion::Enum(conversion))
            | Some(TypeConversion::Union(conversion)) => {
                conversion.copies.iter().map(|copy| &copy.value).collect()
            }
            _ => Vec::new(),
        };
        pending.extend(named(values.into_iter()));
    }
    chain
}

fn named<'a>(values: impl Iterator<Item = &'a ValuePlan>) -> Vec<String> {
    let mut names = Vec::new();
    for value in values {
        match value {
            ValuePlan::Named(key) => names.push(key.clone()),
            ValuePlan::Sequence(value) | ValuePlan::Array(value) | ValuePlan::Optional(value) => {
                names.extend(named([value.as_ref()].into_iter()));
            }
            ValuePlan::Map { key, value } => {
                names.extend(named([key.as_ref(), value.as_ref()].into_iter()));
            }
            ValuePlan::Identity => {}
        }
    }
    names
}

/// Named callees hidden inside sequence, map, or optional members.
fn wrapped_named(step: &StepPlan, key: &str) -> Vec<String> {
    let Some(conversion) = struct_conversion(step, key) else {
        return Vec::new();
    };
    let mut names = Vec::new();
    for copy in &conversion.copies {
        let mut stack = vec![&copy.value];
        while let Some(value) = stack.pop() {
            match value {
                ValuePlan::Named(key) => names.push(key.clone()),
                ValuePlan::Sequence(value)
                | ValuePlan::Array(value)
                | ValuePlan::Optional(value) => stack.push(value),
                ValuePlan::Map { key, value } => {
                    stack.push(key);
                    stack.push(value);
                }
                ValuePlan::Identity => {}
            }
        }
        // Direct callees thread arguments; only wrapped ones are lost.
        if let ValuePlan::Named(_) = &copy.value {
            names.pop();
        }
    }
    names
}

/// Upcast struct conversion of one type key in this step.
fn struct_conversion<'a>(step: &'a StepPlan, key: &str) -> Option<&'a ConversionPlan> {
    let conversion = step
        .types
        .iter()
        .find(|ty| ty.key == key)?
        .upcast
        .as_ref()?;
    match conversion {
        TypeConversion::Struct(conversion) => Some(conversion),
        _ => None,
    }
}

/// Canonical key of a single named-model `return` output, resolved against
/// the type keys of the version that declares it.
fn return_key(outputs: &[Member], types: &BTreeMap<String, TypeShape>) -> Option<String> {
    let [output] = outputs else {
        return None;
    };
    if output.name != "return" {
        return None;
    }
    let value = output.value.as_ref()?;
    let hir::TypeSpec::ScopedName(name) = &value.ty else {
        return None;
    };
    let key = resolve_canonical(&value.scope, name, |key| types.contains_key(key))?;
    matches!(types.get(&key), Some(TypeShape::Struct(_))).then_some(key)
}

/// Members one step removes from the operation's return type; their shapes
/// come from the version that still serves them, ready for persistence.
fn return_removed(step: &StepPlan, key: &str) -> Vec<Member> {
    let Some(ty) = step.types.iter().find(|ty| ty.key == key) else {
        return Vec::new();
    };
    let Some(downcast) = ty.downcast.as_ref() else {
        return Vec::new();
    };
    match downcast {
        TypeConversion::Struct(conversion) => conversion.assignments.clone(),
        _ => Vec::new(),
    }
}

/// Derive the update surface for the whole timeline. Only operations whose
/// Head declaration is a `patch` or `put` returning one named model take part;
/// their request members pair with the members the same step removes from the
/// return type, and everything else about the update flow follows from that.
pub(crate) fn plan(steps: &[StepPlan], versions: &[VersionPlan]) -> IdlcResult<Updates> {
    let head = versions
        .last()
        .ok_or_else(|| IdlcError::rpc("version adapters: version timeline is empty"))?;
    let selected = |key: &str| {
        head.surface.operations.get(key).and_then(|operation| {
            matches!(
                operation.http.meta.method,
                HttpMethod::Patch | HttpMethod::Put
            )
            .then(|| return_key(&operation.outputs, &head.surface.types))
            .flatten()
        })
    };
    let mut updates = Updates::default();
    let mut owners: BTreeMap<String, String> = BTreeMap::new();
    let mut bags: BTreeMap<String, Vec<Member>> = BTreeMap::new();
    for step in steps {
        let mut step_updates = StepUpdates::default();
        for plan in &step.operations {
            let Some(head_return) = selected(&plan.key) else {
                continue;
            };
            let chain = chain(step, &plan.upcast);
            let removed = return_key(&plan.target.outputs, &step.target_types)
                .map(|key| return_removed(step, &key))
                .unwrap_or_default();
            let paired_of = |conversion: &ConversionPlan| {
                conversion
                    .ignored
                    .iter()
                    .filter(|member| removed.iter().any(|removed| removed.wire == member.wire))
                    .cloned()
                    .collect::<Vec<_>>()
            };
            let mut current: BTreeSet<String> = chain
                .iter()
                .filter(|key| {
                    struct_conversion(step, key)
                        .is_some_and(|conversion| !conversion.ignored.is_empty())
                })
                .cloned()
                .collect();
            let mut paired = BTreeMap::new();
            for key in &chain {
                let Some(conversion) = struct_conversion(step, key) else {
                    continue;
                };
                let members = paired_of(conversion);
                if members.is_empty() {
                    continue;
                }
                current.insert(key.clone());
                paired.insert(key.clone(), members);
            }
            loop {
                let mut grew = false;
                for key in &chain {
                    if current.contains(key) {
                        continue;
                    }
                    let Some(conversion) = step
                        .types
                        .iter()
                        .find(|ty| &ty.key == key)
                        .and_then(|ty| ty.upcast.as_ref())
                    else {
                        continue;
                    };
                    let callees = match conversion {
                        TypeConversion::Struct(conversion) => {
                            named(conversion.copies.iter().map(|copy| &copy.value))
                        }
                        TypeConversion::Alias(value) => named([value].into_iter()),
                        TypeConversion::Union(conversion) => {
                            named(conversion.copies.iter().map(|copy| &copy.value))
                        }
                        TypeConversion::Enum(_) | TypeConversion::Hook => Vec::new(),
                    };
                    if callees.iter().any(|callee| current.contains(callee)) {
                        if !matches!(conversion, TypeConversion::Struct(_)) {
                            return Err(IdlcError::rpc(format!(
                                "version adapters: update request chain for {} routes current through the {key} {} conversion; give the retired members a struct path",
                                plan.key,
                                match conversion {
                                    TypeConversion::Alias(_) => "alias",
                                    TypeConversion::Union(_) => "union",
                                    _ => "non-struct",
                                }
                            )));
                        }
                        current.insert(key.clone());
                        grew = true;
                    }
                }
                if !grew {
                    break;
                }
            }
            for key in &chain {
                for hidden in wrapped_named(step, key) {
                    if current.contains(&hidden) {
                        return Err(IdlcError::rpc(format!(
                            "version adapters: update request chain for {} reaches the current conversion {hidden} through a sequence, map, or optional member; retire the member at the struct level instead",
                            plan.key
                        )));
                    }
                }
            }
            let active = !plan.upcast.ignored.is_empty() || !current.is_empty();
            if active {
                step_updates.current.extend(current.iter().cloned());
                step_updates.current.insert(plan.key.clone());
                step_updates
                    .owner
                    .insert(plan.key.clone(), plan.key.clone());
                for key in &current {
                    step_updates.owner.insert(key.clone(), plan.key.clone());
                }
                updates
                    .operations
                    .entry(plan.key.clone())
                    .or_insert_with(|| OperationUpdate {
                        return_key: head_return.clone(),
                        stored: Vec::new(),
                    });
            }
            let envelope_pair = paired_of(&plan.upcast);
            if !envelope_pair.is_empty() {
                paired.insert(plan.key.clone(), envelope_pair);
            }
            for (key, members) in paired {
                let owner = owners
                    .entry(key.clone())
                    .or_insert_with(|| plan.key.clone());
                if owner != &plan.key {
                    return Err(IdlcError::rpc(format!(
                        "version adapters: operations {} and {} share the request conversion for {key}; update adapters need operation-specific request types",
                        owner, plan.key
                    )));
                }
                step_updates.paired.insert(key.clone(), members.clone());
                Updates::insert(bags.entry(plan.key.clone()).or_default(), &members);
            }
        }
        updates.steps.push(step_updates);
    }
    for (key, update) in &mut updates.operations {
        update.stored = bags.remove(key).unwrap_or_default();
    }
    Ok(updates)
}
