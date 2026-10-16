//! Serializable version-map models and internal comparison surfaces.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Stable, JSON-serializable version history facts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VersionMap {
    /// Schema version for readers of this file.
    pub format: u32,
    /// Contract filename supplied by the caller.
    pub contract: String,
    /// Declared API versions in source order.
    pub versions: Vec<MapVersion>,
    /// Shallow differences for every adjacent pair.
    pub steps: Vec<MapStep>,
}

/// One declared version in a [`VersionMap`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MapVersion {
    /// Enum member name.
    pub id: String,
    /// Wire/document value.
    pub value: String,
}

/// One adjacent version change set.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MapStep {
    /// Earlier version member name.
    pub from: String,
    /// Later version member name.
    pub to: String,
    /// Changed data declarations, keyed by qualified name.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub types: BTreeMap<String, TypeChange>,
    /// Changed operations, keyed by `Interface.operation`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub operations: BTreeMap<String, Change>,
}

/// A changed data declaration and its direct members.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TypeChange {
    /// `added`, `removed`, or `changed`.
    pub change: ChangeKind,
    /// Declaration kind for additions and changes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Declaration-level facts for types whose shape is not represented by
    /// members alone (for example typedefs, natives and union switches).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<BTreeMap<String, Value>>,
    /// Direct member facts; references are intentionally not followed here.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub members: BTreeMap<String, Change>,
}

/// One added, removed, or changed fact.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Change {
    /// The kind of change.
    pub change: ChangeKind,
    /// Earlier shallow descriptor for a changed or removed item.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<BTreeMap<String, Value>>,
    /// Later shallow descriptor for a changed or added item.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<BTreeMap<String, Value>>,
}

/// The only change classifications emitted by the core.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ChangeKind {
    /// The item exists only in the later version.
    Added,
    /// The item exists only in the earlier version.
    Removed,
    /// The same pairing key exists with a different shallow declaration.
    Changed,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct TypeSurface {
    pub(super) kind: String,
    pub(super) details: BTreeMap<String, Value>,
    pub(super) members: BTreeMap<String, BTreeMap<String, Value>>,
}

pub(super) type TypeParts<'a> = (
    &'a str,
    &'a str,
    BTreeMap<String, Value>,
    BTreeMap<String, BTreeMap<String, Value>>,
);

#[derive(Debug, Clone, PartialEq)]
pub(super) struct OperationSurface {
    pub(super) descriptor: BTreeMap<String, Value>,
    pub(super) params: BTreeMap<String, BTreeMap<String, Value>>,
}
