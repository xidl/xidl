//! Compares adjacent collected surfaces without following references.

use super::helpers::object;
use super::model::{Change, ChangeKind, OperationSurface, TypeChange, TypeSurface};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn compare_types(
    from: &BTreeMap<String, TypeSurface>,
    to: &BTreeMap<String, TypeSurface>,
) -> BTreeMap<String, TypeChange> {
    let keys = from
        .keys()
        .chain(to.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    keys.into_iter()
        .filter_map(|key| match (from.get(&key), to.get(&key)) {
            (None, Some(value)) => Some((
                key,
                TypeChange {
                    change: ChangeKind::Added,
                    kind: Some(value.kind.clone()),
                    details: (!value.details.is_empty()).then(|| value.details.clone()),
                    members: BTreeMap::new(),
                },
            )),
            (Some(value), None) => Some((
                key,
                TypeChange {
                    change: ChangeKind::Removed,
                    kind: Some(value.kind.clone()),
                    details: (!value.details.is_empty()).then(|| value.details.clone()),
                    members: BTreeMap::new(),
                },
            )),
            (Some(left), Some(right)) => {
                let members = compare_values(&left.members, &right.members);
                (left.kind != right.kind || left.details != right.details || !members.is_empty())
                    .then_some((
                        key,
                        TypeChange {
                            change: ChangeKind::Changed,
                            kind: Some(right.kind.clone()),
                            details: (!right.details.is_empty()).then(|| right.details.clone()),
                            members,
                        },
                    ))
            }
            (None, None) => None,
        })
        .collect()
}

pub(crate) fn compare_operations(
    from: &BTreeMap<String, OperationSurface>,
    to: &BTreeMap<String, OperationSurface>,
) -> BTreeMap<String, Change> {
    let keys = from
        .keys()
        .chain(to.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    keys.into_iter()
        .filter_map(|key| match (from.get(&key), to.get(&key)) {
            (None, Some(value)) => Some((
                key,
                change(ChangeKind::Added, None, Some(operation_descriptor(value))),
            )),
            (Some(value), None) => Some((
                key,
                change(ChangeKind::Removed, Some(operation_descriptor(value)), None),
            )),
            (Some(left), Some(right)) => {
                let mut from_descriptor = left.descriptor.clone();
                from_descriptor.insert(
                    "parameters".to_string(),
                    Value::Object(object(&left.params)),
                );
                let mut to_descriptor = right.descriptor.clone();
                to_descriptor.insert(
                    "parameters".to_string(),
                    Value::Object(object(&right.params)),
                );
                (from_descriptor != to_descriptor).then_some((
                    key,
                    change(
                        ChangeKind::Changed,
                        Some(from_descriptor),
                        Some(to_descriptor),
                    ),
                ))
            }
            (None, None) => None,
        })
        .collect()
}

fn operation_descriptor(value: &OperationSurface) -> BTreeMap<String, Value> {
    let mut descriptor = value.descriptor.clone();
    if !value.params.is_empty() {
        descriptor.insert(
            "parameters".to_string(),
            Value::Object(object(&value.params)),
        );
    }
    descriptor
}

fn compare_values(
    from: &BTreeMap<String, BTreeMap<String, Value>>,
    to: &BTreeMap<String, BTreeMap<String, Value>>,
) -> BTreeMap<String, Change> {
    let keys = from
        .keys()
        .chain(to.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    keys.into_iter()
        .filter_map(|key| match (from.get(&key), to.get(&key)) {
            (None, Some(value)) => {
                Some((key, change(ChangeKind::Added, None, Some(value.clone()))))
            }
            (Some(value), None) => {
                Some((key, change(ChangeKind::Removed, Some(value.clone()), None)))
            }
            (Some(left), Some(right)) => (left != right).then_some((
                key,
                change(ChangeKind::Changed, Some(left.clone()), Some(right.clone())),
            )),
            (None, None) => None,
        })
        .collect()
}

fn change(
    kind: ChangeKind,
    from: Option<BTreeMap<String, Value>>,
    to: Option<BTreeMap<String, Value>>,
) -> Change {
    Change {
        change: kind,
        from,
        to,
    }
}
