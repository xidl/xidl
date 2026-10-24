//! Translate direct member facts into the agreed asymmetric hook rules.

use super::model::*;
use crate::error::{IdlcError, IdlcResult};
use std::collections::{BTreeMap, BTreeSet};
use xidl_parser::versioning::map::{Change, ChangeKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Direction {
    Upcast,
    Downcast,
}

/// Fields require aggregate decisions; variants decide only changed alternatives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum MemberSetKind {
    Fields,
    Variants,
}

pub(super) struct MemberClassifier<'a> {
    pub source: &'a [Member],
    pub target: &'a [Member],
    pub facts: &'a BTreeMap<String, Change>,
    pub direction: Direction,
    pub kind: MemberSetKind,
    pub context: &'a str,
}

impl MemberClassifier<'_> {
    pub(super) fn plan(&self) -> IdlcResult<ConversionPlan> {
        let mut plan = ConversionPlan::default();
        let upward = self.direction == Direction::Upcast;
        let paired = |source: &Member, target: &Member| {
            if source.key != target.key {
                // A `@replaces` link pairs the retirement with its successor,
                // so the value copies mechanically in both directions.
                let (earlier, later) = if upward {
                    (source, target)
                } else {
                    (target, source)
                };
                return self.replaces(later) == Some(earlier.name.as_str());
            }
            match self.facts.get(&source.key) {
                None => true,
                Some(change)
                    if self.kind == MemberSetKind::Variants
                        && change.change == ChangeKind::Changed =>
                {
                    let old = change
                        .from
                        .as_ref()
                        .and_then(|value| value.get("declaration_id"))
                        .and_then(serde_json::Value::as_str);
                    let new = change
                        .to
                        .as_ref()
                        .and_then(|value| value.get("declaration_id"))
                        .and_then(serde_json::Value::as_str);
                    old.is_some() && old == new && source.name == target.name
                }
                Some(_) => false,
            }
        };
        let changed_source = self
            .source
            .iter()
            .filter(|source| !self.target.iter().any(|target| paired(source, target)))
            .collect::<Vec<_>>();
        let changed_target = self
            .target
            .iter()
            .filter(|target| !self.source.iter().any(|source| paired(source, target)))
            .collect::<Vec<_>>();
        let replacement = !changed_source.is_empty() && !changed_target.is_empty();
        let mut covered = BTreeSet::new();
        for source in self.source {
            if let Some(target) = self.target.iter().find(|target| paired(source, target)) {
                let linked = source.key != target.key;
                if !Self::same_shape(source, target)? {
                    if linked {
                        return Err(IdlcError::rpc(format!(
                            "version adapters {}: {} and {} are linked by replaces but differ in shape; a rename cannot also reshape",
                            self.context, source.key, target.key
                        )));
                    }
                    return Err(IdlcError::rpc(format!(
                        "version adapters {}: version_map omitted the change to {}",
                        self.context, source.key
                    )));
                }
                if !linked && source.name != target.name {
                    return Err(IdlcError::rpc(format!(
                        "version adapters {}: version_map omitted the change to {}",
                        self.context, source.key
                    )));
                }
                covered.insert(target.key.clone());
                plan.copies.push(FieldCopy {
                    source: source.clone(),
                    target: target.clone(),
                    value: ValuePlan::Identity,
                });
            }
        }
        if self.kind == MemberSetKind::Variants {
            plan.consumed = changed_source.into_iter().cloned().collect();
            return Ok(plan);
        }
        if upward {
            plan.ignored = changed_source.into_iter().cloned().collect();
        }
        for target in changed_target {
            // Downcast removed members always decide: storage or OMIT keeps the
            // old contract readable. Upcast additions stay optional when legal.
            let optional = target.value.as_ref().is_some_and(|value| value.optional);
            if !upward || !optional || replacement {
                covered.insert(target.key.clone());
                plan.assignments.push(target.clone());
            }
        }
        for target in self.target {
            if covered.contains(&target.key) {
                continue;
            }
            if !target.value.as_ref().is_some_and(|value| value.optional) {
                return Err(IdlcError::rpc(format!(
                    "version adapters {}: required member {} is unassigned",
                    self.context, target.name
                )));
            }
            plan.absences.push(target.clone());
        }
        Ok(plan)
    }

    /// The earlier member name the later member takes over from, as the
    /// version map recorded it.
    fn replaces(&self, later: &Member) -> Option<&str> {
        self.facts
            .get(&later.key)
            .and_then(|change| change.to.as_ref())
            .and_then(|to| to.get("replaces"))
            .and_then(serde_json::Value::as_str)
    }

    pub(super) fn same_shape(source: &Member, target: &Member) -> IdlcResult<bool> {
        match (&source.value, &target.value) {
            (None, None) => Ok(true),
            (Some(source), Some(target)) => {
                let array = |value: &TypeUse| match &value.declarator {
                    xidl_parser::hir::Declarator::ArrayDeclarator(array) => Some(array.len.clone()),
                    _ => None,
                };
                Ok(source.representation == target.representation
                    && source.optional == target.optional
                    && serde_json::to_value(&source.ty)? == serde_json::to_value(&target.ty)?
                    && serde_json::to_value(array(source))? == serde_json::to_value(array(target))?)
            }
            _ => Ok(false),
        }
    }
}
