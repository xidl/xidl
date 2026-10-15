//! Derivation of the version-adapter plan.
//!
//! The adapter stage is the one stage allowed to see several versions at
//! once: it pairs every adjacent version step by declaration identity and
//! canonical wire name, classifies each movement per the #324 rules table,
//! and reduces reachability from the operations both versions share. The
//! plan is pure data — language renderers turn it into hooks and mechanical
//! conversions, and per-row unit tests pin the table without any rendering.

pub(crate) mod render;
pub(crate) mod surface;

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};

use crate::error::{IdlcError, IdlcResult};
use xidl_parser::hir;
use xidl_parser::versioning::{MemberInterval, VersionDef};

pub(crate) use surface::{MemberSurface, ParameterSurface, TypeKind, TypeSurface, VersionSurface};

/// The adapter plan over the declared timeline.
#[derive(Debug, Clone)]
pub(crate) struct AdapterPlan {
    pub(crate) steps: Vec<StepPlan>,
}

/// One version step: the older version paired against the newer one.
#[derive(Debug, Clone)]
pub(crate) struct StepPlan {
    pub(crate) from: VersionDef,
    pub(crate) to: VersionDef,
    /// Reachable type conversions, in declaration order.
    pub(crate) types: Vec<TypePlan>,
    /// Operations both versions declare, in declaration order.
    pub(crate) operations: Vec<OperationPlan>,
}

/// One paired type and its two directions.
#[derive(Debug, Clone)]
pub(crate) struct TypePlan {
    pub(crate) name: String,
    pub(crate) kind: TypeKind,
    /// Older request data becomes newer: `V → V+1`.
    pub(crate) upcast: Option<ConversionPlan>,
    /// Newer result data becomes older: `V+1 → V`.
    pub(crate) downcast: Option<ConversionPlan>,
}

/// One direction of one type's conversion.
#[derive(Debug, Clone)]
pub(crate) struct ConversionPlan {
    /// Mechanical member copies, in source declaration order.
    pub(crate) copies: Vec<FieldCopy>,
    /// Set when the direction needs decisions; the renderer emits the hook
    /// signature and the application writes the body.
    pub(crate) hook: Option<HookPlan>,
}

/// One mechanical member copy.
#[derive(Debug, Clone)]
pub(crate) struct FieldCopy {
    pub(crate) from: MemberSurface,
    pub(crate) to: MemberSurface,
}

/// A hook: the decisions the application must make.
#[derive(Debug, Clone)]
pub(crate) struct HookPlan {
    /// Struct hooks: the fields the hook result must supply — a required
    /// member the source shape cannot carry, or the `IGNORED` marker
    /// (`ignored`) proving a removed member was consciously dropped. Enum
    /// hooks leave this empty.
    pub(crate) produce: Vec<HookField>,
    /// Enum hooks: the values the hook reads — the ones removed in the step
    /// on upcast, the ones added on downcast. Struct hooks leave this empty.
    pub(crate) consumed: Vec<MemberSurface>,
}

/// One field of a hook's result type.
#[derive(Debug, Clone)]
pub(crate) struct HookField {
    pub(crate) member: MemberSurface,
    /// `true` when the only legal value is the `IGNORED` marker.
    pub(crate) ignored: bool,
}

/// One operation both versions of a step declare.
///
/// Requests only ever flow upward, so the parameters carry an upcast plan;
/// responses flow downward through the return type's own plan.
#[derive(Clone, Debug)]
pub(crate) struct OperationPlan {
    /// Interface the operation belongs to.
    pub(crate) interface: String,
    pub(crate) name: String,
    /// The request parameters form a synthetic struct, converted like any
    /// other type: mechanical copies plus one upcast hook.
    pub(crate) upcast: Option<ConversionPlan>,
    /// The head version's return type; the downcast entry points project it.
    pub(crate) return_type: Option<hir::TypeSpec>,
    /// Exceptions the newer version raises that the older one cannot,
    /// driving the per-operation error downcast hook.
    pub(crate) raises_added: Vec<String>,
}

/// Derives the adapter plan for the given timeline slice.
///
/// `versions` is the adjacent version run to serve, in timeline order, Head
/// last; `surfaces` is keyed by version module ident; `intervals` come from
/// the unprojected specification. Fails when a non-Head version still
/// exposes an operation Head removed: phase 1 cannot serve it, and a silent
/// 404 is worse than a loud generation error.
pub(crate) fn derive(
    surfaces: &BTreeMap<String, VersionSurface>,
    versions: &[VersionDef],
    intervals: &[MemberInterval],
) -> IdlcResult<AdapterPlan> {
    let head = versions
        .last()
        .ok_or_else(|| IdlcError::fmt("version adapters need at least one declared version"))?;
    let head_surface = surface_for(surfaces, &head.ident)?;
    for version in versions {
        if version.ident == head.ident {
            continue;
        }
        let surface = surface_for(surfaces, &version.ident)?;
        for (interface_name, interface) in &surface.interfaces {
            let head_interface = head_surface.interfaces.get(interface_name);
            for name in interface.operations.keys() {
                let served = head_interface.is_some_and(|head| head.operations.contains_key(name));
                if !served {
                    return Err(IdlcError::fmt(format!(
                        "version {} still exposes operation {}, which the head version {} removed; phase 1 cannot serve removed operations, map them to a head operation instead",
                        version.value, name, head.value
                    )));
                }
            }
        }
    }

    let mut steps = Vec::new();
    for (index, pair) in versions.windows(2).enumerate() {
        let (from_def, to_def) = (&pair[0], &pair[1]);
        let from = surface_for(surfaces, &from_def.ident)?;
        let to = surface_for(surfaces, &to_def.ident)?;
        let types = pair_types(from, to, intervals, index, index + 1);
        let operations = pair_operations(from, to, intervals, index, index + 1);
        steps.push(StepPlan {
            from: from_def.clone(),
            to: to_def.clone(),
            types,
            operations,
        });
    }
    Ok(AdapterPlan { steps })
}

/// Pairs every type the two versions share, keeping only conversions whose
/// type the step's operations actually reach.
fn pair_types(
    from: &VersionSurface,
    to: &VersionSurface,
    intervals: &[MemberInterval],
    from_index: usize,
    to_index: usize,
) -> Vec<TypePlan> {
    let conversions: BTreeMap<String, (Option<ConversionPlan>, Option<ConversionPlan>)> = from
        .types
        .iter()
        .filter_map(|(name, from_type)| {
            let to_type = to.types.get(name)?;
            Some((
                name.clone(),
                (
                    convert_direction(
                        Direction::Upcast,
                        from_type,
                        to_type,
                        intervals,
                        from_index,
                        to_index,
                    ),
                    convert_direction(
                        Direction::Downcast,
                        to_type,
                        from_type,
                        intervals,
                        to_index,
                        from_index,
                    ),
                ),
            ))
        })
        .collect();
    let plans: Vec<TypePlan> = conversions
        .iter()
        .map(|(name, (upcast, downcast))| TypePlan {
            name: name.clone(),
            kind: from.types[name].kind,
            upcast: upcast.clone(),
            downcast: downcast.clone(),
        })
        .collect();

    let up_reachable = reachable(&conversions, &upcast_roots(from, to));
    let down_reachable = reachable(&conversions, &downcast_roots(from, to));
    plans
        .into_iter()
        .filter_map(|mut plan| {
            if !up_reachable.contains(&plan.name) {
                plan.upcast = None;
            }
            if !down_reachable.contains(&plan.name) {
                plan.downcast = None;
            }
            (plan.upcast.is_some() || plan.downcast.is_some()).then_some(plan)
        })
        .collect()
}

/// Types the upcast conversions start from: input types of the operations
/// both versions share.
fn upcast_roots(from: &VersionSurface, to: &VersionSurface) -> BTreeSet<String> {
    let mut roots = BTreeSet::new();
    for (interface_name, interface) in &from.interfaces {
        let Some(to_interface) = to.interfaces.get(interface_name) else {
            continue;
        };
        for (name, operation) in &interface.operations {
            if !to_interface.operations.contains_key(name) {
                continue;
            }
            roots.extend(scoped_roots(
                operation
                    .params
                    .iter()
                    .map(|parameter| parameter.ty.clone()),
            ));
        }
    }
    roots
}

/// Types the downcast conversions start from: output and raised exception
/// types of the operations both versions share.
fn downcast_roots(from: &VersionSurface, to: &VersionSurface) -> BTreeSet<String> {
    let mut roots = BTreeSet::new();
    for (interface_name, interface) in &from.interfaces {
        let Some(to_interface) = to.interfaces.get(interface_name) else {
            continue;
        };
        for (name, operation) in &interface.operations {
            if !to_interface.operations.contains_key(name) {
                continue;
            }
            if let Some(return_type) = &operation.return_type {
                roots.extend(scoped_roots(std::iter::once(return_type.clone())));
            }
            roots.extend(operation.raises.iter().cloned());
        }
    }
    roots
}

fn scoped_roots(types: impl Iterator<Item = hir::TypeSpec>) -> BTreeSet<String> {
    types
        .filter_map(|ty| match ty {
            hir::TypeSpec::ScopedName(scoped) => scoped.name.last().cloned(),
            _ => None,
        })
        .collect()
}

/// Follows mechanical copies outward from the roots; a hooked member's
/// subtree is the hook's business, not a conversion of its own.
fn reachable(
    conversions: &BTreeMap<String, (Option<ConversionPlan>, Option<ConversionPlan>)>,
    roots: &BTreeSet<String>,
) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut queue: Vec<String> = roots
        .iter()
        .filter(|name| conversions.contains_key(*name))
        .cloned()
        .collect();
    while let Some(name) = queue.pop() {
        if !seen.insert(name.clone()) {
            continue;
        }
        let Some((upcast, downcast)) = conversions.get(&name) else {
            continue;
        };
        for direction in [upcast, downcast] {
            let Some(plan) = direction else {
                continue;
            };
            for copy in &plan.copies {
                if let Some(hir::TypeSpec::ScopedName(scoped)) = &copy.from.ty {
                    if let Some(target) = scoped.name.last() {
                        if conversions.contains_key(target) && !seen.contains(target) {
                            queue.push(target.clone());
                        }
                    }
                }
            }
        }
    }
    seen
}

/// The conversion direction; it decides how asymmetric movements classify.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Upcast,
    Downcast,
}

/// Classifies one direction: `source` is the version whose data the
/// conversion reads, `target` the version whose shape it produces.
///
/// A member present on one side only is symmetric in interval terms but
/// asymmetric in meaning: reading the older shape it is a removal, reading
/// the newer shape it is an addition, and the rules table differs per case.
fn convert_direction(
    direction: Direction,
    source: &TypeSurface,
    target: &TypeSurface,
    intervals: &[MemberInterval],
    source_index: usize,
    target_index: usize,
) -> Option<ConversionPlan> {
    let upward = direction == Direction::Upcast;
    let mut copies = Vec::new();
    let mut produce: Vec<HookField> = Vec::new();
    let mut consumed: Vec<MemberSurface> = Vec::new();
    let mut needs_hook = false;

    for source_member in &source.members {
        let in_source = exists_in(intervals, &source_member.decl_path, source_index);
        let in_target = exists_in(intervals, &source_member.decl_path, target_index);
        if in_source && in_target {
            if let Some(target_member) = member_by_wire(&target.members, &source_member.wire_name) {
                if target_member.decl_path == source_member.decl_path
                    && target_member.location == source_member.location
                    && written_matches(&source_member.ty, &target_member.ty)
                {
                    copies.push(FieldCopy {
                        from: source_member.clone(),
                        to: target_member,
                    });
                } else {
                    // Two distinct declarations — or one declaration whose
                    // written type changed — share the wire name: both
                    // directions need a decision, and the value the retiring
                    // side carried is explicitly accounted for.
                    needs_hook = true;
                    if source.kind == TypeKind::Enum {
                        consumed.push(source_member.clone());
                        consumed.push(target_member);
                    } else if upward {
                        produce.push(HookField {
                            member: target_member,
                            ignored: false,
                        });
                        produce.push(HookField {
                            member: source_member.clone(),
                            ignored: true,
                        });
                    } else {
                        produce.push(HookField {
                            member: target_member,
                            ignored: false,
                        });
                    }
                }
            }
            continue;
        }
        if in_source && !in_target {
            // The source shape carries a member the target version does not
            // declare. Reading the older shape that is a removal: the upcast
            // hook must acknowledge the drop, the downcast cannot receive
            // the value at all. Reading the newer shape it is an addition:
            // dropped mechanically on the way down, except enum values the
            // older version cannot represent.
            match (upward, source.kind) {
                (true, TypeKind::Enum) => {
                    needs_hook = true;
                    consumed.push(source_member.clone());
                }
                (true, TypeKind::Members) => {
                    needs_hook = true;
                    produce.push(HookField {
                        member: source_member.clone(),
                        ignored: true,
                    });
                }
                (false, TypeKind::Enum) => {
                    needs_hook = true;
                    consumed.push(source_member.clone());
                }
                (false, TypeKind::Members) => {}
            }
        }
    }
    for target_member in &target.members {
        let in_source = exists_in(intervals, &target_member.decl_path, source_index);
        let in_target = exists_in(intervals, &target_member.decl_path, target_index);
        if in_source || !in_target {
            continue;
        }
        // The target shape requires a member the source version does not
        // declare. Producing the newer shape that is an addition; producing
        // the older shape that is a removal the source cannot carry.
        let required = !target_member.optional;
        match (upward, source.kind) {
            (true, TypeKind::Enum) => {}
            (true, TypeKind::Members) if required => {
                needs_hook = true;
                produce.push(HookField {
                    member: target_member.clone(),
                    ignored: false,
                });
            }
            (true, TypeKind::Members) => {}
            (false, TypeKind::Enum) => {}
            (false, TypeKind::Members) if required => {
                needs_hook = true;
                produce.push(HookField {
                    member: target_member.clone(),
                    ignored: false,
                });
            }
            (false, TypeKind::Members) => {}
        }
    }
    if copies.is_empty() && !needs_hook {
        return None;
    }
    let hook = needs_hook.then_some(HookPlan { produce, consumed });
    Some(ConversionPlan { copies, hook })
}

/// Structural identity of a written type: shallow by design, because a
/// referenced type evolves independently and its own pairing handles that.
fn written_matches(from: &Option<hir::TypeSpec>, to: &Option<hir::TypeSpec>) -> bool {
    match (from, to) {
        (Some(from), Some(to)) => type_key(from) == type_key(to),
        (None, None) => true,
        _ => false,
    }
}

fn type_key(ty: &hir::TypeSpec) -> String {
    serde_json::to_string(ty).unwrap_or_default()
}

fn member_by_wire(members: &[MemberSurface], wire_name: &str) -> Option<MemberSurface> {
    members
        .iter()
        .find(|member| member.wire_name == wire_name)
        .cloned()
}

fn exists_in(intervals: &[MemberInterval], path: &str, index: usize) -> bool {
    intervals
        .iter()
        .any(|interval| interval.path == path && interval.from <= index && index < interval.until)
}

fn surface_for<'a>(
    surfaces: &'a BTreeMap<String, VersionSurface>,
    ident: &str,
) -> IdlcResult<&'a VersionSurface> {
    surfaces.get(ident).ok_or_else(|| {
        IdlcError::fmt(format!(
            "version module {ident} is missing from the projected specification"
        ))
    })
}

/// Pairs the operations both versions declare.
fn pair_operations(
    from: &VersionSurface,
    to: &VersionSurface,
    intervals: &[MemberInterval],
    from_index: usize,
    to_index: usize,
) -> Vec<OperationPlan> {
    let mut plans = Vec::new();
    for (interface_name, from_interface) in &from.interfaces {
        let Some(to_interface) = to.interfaces.get(interface_name) else {
            continue;
        };
        for (name, from_operation) in &from_interface.operations {
            let Some(to_operation) = to_interface.operations.get(name) else {
                continue;
            };
            let from_raises: BTreeSet<&str> =
                from_operation.raises.iter().map(String::as_str).collect();
            let raises_added = to_operation
                .raises
                .iter()
                .filter(|exception| !from_raises.contains(exception.as_str()))
                .cloned()
                .collect();
            let from_params = synthetic_struct(&from_operation.params);
            let to_params = synthetic_struct(&to_operation.params);
            let upcast = convert_direction(
                Direction::Upcast,
                &from_params,
                &to_params,
                intervals,
                from_index,
                to_index,
            );
            plans.push(OperationPlan {
                interface: interface_name.clone(),
                name: name.clone(),
                upcast,
                return_type: to_operation.return_type.clone(),
                raises_added,
            });
        }
    }
    plans
}

/// The rules table treats an operation's request parameters as one synthetic
/// struct, so they classify exactly like members do.
fn synthetic_struct(params: &[ParameterSurface]) -> TypeSurface {
    TypeSurface {
        kind: TypeKind::Members,
        members: params
            .iter()
            .map(|parameter| MemberSurface {
                decl_path: parameter.decl_path.clone(),
                ident: parameter.ident.clone(),
                wire_name: parameter.wire_name.clone(),
                ty: Some(parameter.ty.clone()),
                optional: parameter.optional,
                location: Some(parameter.location),
            })
            .collect(),
    }
}
