//! Normalization of generated OpenAPI documents for release freezing.
//!
//! Freezing compares documents as structured data, so prose and presentation
//! churn must not read as a wire change: documentation keywords disappear at
//! their OpenAPI keyword positions, unreferenced components are pruned, and
//! set-like arrays get a canonical order. Name-keyed maps — `properties`,
//! component collections, response header maps — carry member names, never
//! keywords, and their keys are never stripped.

use std::collections::BTreeSet;

use serde_json::Value;

/// Documentation keywords stripped wherever they appear as OpenAPI keywords.
const STRIP: &[&str] = &[
    "description",
    "summary",
    "example",
    "deprecated",
    "servers",
    "openapi",
];

/// Object keys whose members are user-chosen names instead of keywords.
const NAME_MAPS: &[&str] = &["properties", "schemas", "headers", "securitySchemes"];

/// Normalizes one generated OpenAPI document in place.
pub(crate) fn normalize_document(document: &mut Value) {
    strip_keywords(document, false);
    sort_collections(document);
    prune_components(document);
}

/// Strips documentation keywords, never the keys of name-keyed maps.
fn strip_keywords(value: &mut Value, in_name_map: bool) {
    match value {
        Value::Object(map) => {
            let stripped: Vec<String> = if in_name_map {
                Vec::new()
            } else {
                map.keys()
                    .filter(|key| STRIP.contains(&key.as_str()))
                    .cloned()
                    .collect()
            };
            for key in stripped {
                map.remove(&key);
            }
            for (key, child) in map.iter_mut() {
                strip_keywords(child, NAME_MAPS.contains(&key.as_str()));
            }
        }
        Value::Array(items) => {
            for item in items {
                strip_keywords(item, false);
            }
        }
        _ => {}
    }
}

/// Gives set-like arrays a canonical order and sorts parameters by identity.
fn sort_collections(document: &mut Value) {
    match document {
        Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                match key.as_str() {
                    "required" | "tags" => sort_string_array(child),
                    "parameters" => sort_parameters(child),
                    _ => sort_collections(child),
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                sort_collections(item);
            }
        }
        _ => {}
    }
}

fn sort_string_array(value: &mut Value) {
    if let Value::Array(items) = value {
        items.sort_by_key(|item| match item {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        });
    }
}

fn sort_parameters(value: &mut Value) {
    if let Value::Array(items) = value {
        items.sort_by_key(|parameter| {
            let location = parameter
                .get("in")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let name = parameter
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            (location, name)
        });
    }
}

/// Drops components nothing references, to a fixpoint.
fn prune_components(document: &mut Value) {
    let mut keep = refs_outside_components(document);
    loop {
        let mut next = keep.clone();
        for name in &keep {
            next.extend(schema_refs(document, name));
        }
        if next == keep {
            break;
        }
        keep = next;
    }
    if let Some(schemas) = document
        .get_mut("components")
        .and_then(|components| components.get_mut("schemas"))
        .and_then(Value::as_object_mut)
    {
        let stale: Vec<String> = schemas
            .keys()
            .filter(|name| !keep.contains(*name))
            .cloned()
            .collect();
        for name in stale {
            schemas.remove(&name);
        }
    }
}

/// Collects every `#/components/...` reference outside the components section.
fn refs_outside_components(value: &Value) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if key != "components" {
                    found.extend(collect_refs(child, false));
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                found.extend(refs_outside_components(item));
            }
        }
        _ => {}
    }
    found
}

/// Collects references inside one value; `skip_components` suppresses refs
/// under nested `components` sections.
fn collect_refs(value: &Value, skip_components: bool) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if key == "$ref" {
                    if let Some(name) = component_ref_name(child.as_str().unwrap_or_default()) {
                        found.insert(name);
                    }
                } else if !(skip_components && key == "components") {
                    found.extend(collect_refs(child, skip_components));
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                found.extend(collect_refs(item, skip_components));
            }
        }
        _ => {}
    }
    found
}

/// References one kept schema makes; only schemas reference schemas.
fn schema_refs(document: &Value, name: &str) -> BTreeSet<String> {
    document
        .get("components")
        .and_then(|components| components.get("schemas"))
        .and_then(|schemas| schemas.get(name))
        .map(|schema| collect_refs(schema, true))
        .unwrap_or_default()
}

/// The component name a `$ref` target points at, when it is one.
fn component_ref_name(reference: &str) -> Option<String> {
    reference
        .strip_prefix("#/components/")
        .and_then(|rest| rest.split_once('/'))
        .map(|(_, name)| name.to_string())
}

#[cfg(test)]
mod tests;
