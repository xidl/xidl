//! Read and validate the public map at the adapter boundary.

use super::model::Member;
use crate::error::{IdlcError, IdlcResult};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use xidl_parser::hir::ParserProperties;
use xidl_parser::versioning::map::{Change, ChangeKind, VersionMap};

/// Parameters are paired within their request or response direction.
#[derive(Clone, Copy)]
pub(super) enum ParameterSide {
    Request,
    Response,
}

impl ParameterSide {
    fn read(
        self,
        descriptor: Option<&BTreeMap<String, Value>>,
        context: &str,
    ) -> IdlcResult<BTreeMap<String, BTreeMap<String, Value>>> {
        let property = match self {
            Self::Request => "parameters",
            Self::Response => "response_parameters",
        };
        let Some(value) = descriptor.and_then(|descriptor| descriptor.get(property)) else {
            return Ok(BTreeMap::new());
        };
        match self {
            Self::Request => serde_json::from_value(value.clone()).map_err(|error| {
                IdlcError::rpc(format!(
                    "version adapters {context}: malformed request parameters: {error}"
                ))
            }),
            Self::Response => {
                let bindings: BTreeMap<String, Vec<BTreeMap<String, Value>>> =
                    serde_json::from_value(value.clone()).map_err(|error| {
                        IdlcError::rpc(format!(
                            "version adapters {context}: malformed response parameters: {error}"
                        ))
                    })?;
                let mut parameters = BTreeMap::new();
                for descriptor in bindings.into_values().flatten() {
                    let name = descriptor
                        .get("name")
                        .and_then(Value::as_str)
                        .ok_or_else(|| {
                            IdlcError::rpc(format!(
                                "version adapters {context}: response parameter has no source name"
                            ))
                        })?
                        .to_string();
                    if parameters.insert(name.clone(), descriptor).is_some() {
                        return Err(IdlcError::rpc(format!(
                            "version adapters {context}: response source name {name} is ambiguous"
                        )));
                    }
                }
                Ok(parameters)
            }
        }
    }
}

pub(super) struct MapFacts;

impl MapFacts {
    /// Inherited fields use the change recorded on their declaring base. Moving
    /// a wire name between declarations is handled as an explicit decision.
    pub(super) fn type_members(
        types: &BTreeMap<String, xidl_parser::versioning::map::TypeChange>,
        key: &str,
        source: &[Member],
        target: &[Member],
    ) -> BTreeMap<String, Change> {
        let mut facts = types
            .get(key)
            .map(|change| change.members.clone())
            .unwrap_or_default();
        for member in source.iter().chain(target) {
            if let Some(change) = member
                .path
                .last()
                .and_then(|owner| types.get(owner))
                .and_then(|change| change.members.get(&member.key))
            {
                facts.insert(member.key.clone(), change.clone());
            }
        }
        for old in source {
            if let Some(new) = target.iter().find(|member| member.key == old.key)
                && old.path.last() != new.path.last()
            {
                facts.remove(&old.key);
            }
        }
        facts
    }

    pub(super) fn read(props: &ParserProperties) -> IdlcResult<VersionMap> {
        let value = props.get("version_map").ok_or_else(|| {
            IdlcError::rpc(
                "version adapters require props.version_map and a versioned HTTP contract",
            )
        })?;
        let map: VersionMap = serde_json::from_value(value.clone()).map_err(|error| {
            IdlcError::rpc(format!("version adapters: malformed version_map: {error}"))
        })?;
        if map.format != 2 {
            return Err(IdlcError::rpc(format!(
                "version adapters for {}: unsupported version_map format {}",
                map.contract, map.format
            )));
        }
        let mut ids = BTreeSet::new();
        if map.versions.is_empty()
            || map
                .versions
                .iter()
                .any(|version| version.id.is_empty() || !ids.insert(&version.id))
        {
            return Err(IdlcError::rpc(format!(
                "version adapters for {}: version_map must contain unique nonempty version ids",
                map.contract
            )));
        }
        if map.steps.len() != map.versions.len() - 1
            || map
                .steps
                .iter()
                .zip(map.versions.windows(2))
                .any(|(step, pair)| step.from != pair[0].id || step.to != pair[1].id)
        {
            return Err(IdlcError::rpc(format!(
                "version adapters for {}: version_map steps must follow the complete adjacent timeline",
                map.contract
            )));
        }
        Ok(map)
    }

    pub(super) fn validate_members(
        facts: &BTreeMap<String, Change>,
        source: &[Member],
        target: &[Member],
        context: &str,
    ) -> IdlcResult<()> {
        for (key, fact) in facts {
            let old = source.iter().find(|member| member.key == *key);
            let new = target.iter().find(|member| member.key == *key);
            let valid = match fact.change {
                ChangeKind::Added => {
                    old.is_none() && Self::matches(new, fact.to.as_ref()) && fact.from.is_none()
                }
                ChangeKind::Removed => {
                    new.is_none() && Self::matches(old, fact.from.as_ref()) && fact.to.is_none()
                }
                ChangeKind::Changed => {
                    Self::matches(old, fact.from.as_ref()) && Self::matches(new, fact.to.as_ref())
                }
            };
            if !valid {
                return Err(IdlcError::rpc(format!(
                    "version adapters {context}: version_map member {key} disagrees with projected HIR"
                )));
            }
        }
        Ok(())
    }

    pub(super) fn parameters(
        change: Option<&Change>,
        side: ParameterSide,
        context: &str,
    ) -> IdlcResult<BTreeMap<String, Change>> {
        let Some(change) = change else {
            return Ok(BTreeMap::new());
        };
        let from = side.read(change.from.as_ref(), context)?;
        let to = side.read(change.to.as_ref(), context)?;
        let mut facts = BTreeMap::new();
        for key in from.keys().chain(to.keys()) {
            let old = from.get(key);
            let new = to.get(key);
            if old == new {
                continue;
            }
            facts.insert(
                key.clone(),
                Change {
                    change: match (old, new) {
                        (Some(_), Some(_)) => ChangeKind::Changed,
                        (Some(_), None) => ChangeKind::Removed,
                        _ => ChangeKind::Added,
                    },
                    from: old.cloned(),
                    to: new.cloned(),
                },
            );
        }
        Ok(facts)
    }

    fn matches(member: Option<&Member>, descriptor: Option<&BTreeMap<String, Value>>) -> bool {
        match (member, descriptor) {
            (Some(member), Some(descriptor)) => {
                descriptor.get("name").and_then(Value::as_str) == Some(member.name.as_str())
                    && descriptor
                        .get("optional")
                        .and_then(Value::as_bool)
                        .is_none_or(|optional| {
                            member
                                .value
                                .as_ref()
                                .is_some_and(|value| value.optional == optional)
                        })
            }
            _ => false,
        }
    }
}
