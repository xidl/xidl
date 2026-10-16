//! Shared deterministic JSON descriptor helpers for version-map modules.

use serde_json::Value;
use std::collections::BTreeMap;

pub(super) fn descriptor<const N: usize>(entries: [(&str, Value); N]) -> BTreeMap<String, Value> {
    entries
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect()
}

pub(super) fn object(
    values: &BTreeMap<String, BTreeMap<String, Value>>,
) -> serde_json::Map<String, Value> {
    values
        .iter()
        .map(|(key, value)| {
            (
                key.clone(),
                Value::Object(value.clone().into_iter().collect()),
            )
        })
        .collect()
}
