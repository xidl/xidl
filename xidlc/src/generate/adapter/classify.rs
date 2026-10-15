//! Per-direction classification of one version step's members.
//!
//! The rules table is direction-asymmetric: reading the older shape a
//! one-sided member is a removal, reading the newer shape it is an
//! addition, and each case pairs differently.

use xidl_parser::hir;
use xidl_parser::versioning::MemberInterval;

use super::{ConversionPlan, FieldCopy, HookField, HookPlan, MemberSurface, TypeKind, TypeSurface};

/// The conversion direction; it decides how asymmetric movements classify.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Direction {
    Upcast,
    Downcast,
}

/// Classifies one direction: `source` is the version whose data the
/// conversion reads, `target` the version whose shape it produces.
///
/// A member present on one side only is symmetric in interval terms but
/// asymmetric in meaning: reading the older shape it is a removal, reading
/// the newer shape it is an addition, and the rules table differs per case.
pub(crate) fn convert_direction(
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
