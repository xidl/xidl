//! Documents an OpenRPC document reaches through an import.

use std::collections::BTreeSet;

use serde_json::Value;
use std::collections::BTreeMap;
use xidl_parser::hir::ParserProperties;

/// Prefixes of the units a document imports.
///
/// Each unit is generated as its own document, so a `$ref` to one of its
/// schemas names that document instead of the current one.
#[derive(Debug, Default, Clone)]
pub(crate) struct DocumentUnits {
    prefixes: BTreeSet<String>,
}

impl DocumentUnits {
    /// Reads the prefixes of the units `properties` imports.
    pub(crate) fn of(properties: &ParserProperties) -> Self {
        let prefixes = properties
            .get(crate::units::IMPORT_PROPERTY)
            .and_then(Value::as_array)
            .map(|imports| {
                imports
                    .iter()
                    .filter_map(|import| import.get("prefix"))
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        Self { prefixes }
    }

    /// Rewrites every `$ref` that names a schema of another document.
    ///
    /// A schema of the current document keeps its local `$ref`. A name whose
    /// first segment is an imported unit can only come from that unit, because
    /// the unit graph rejects a reference the imported unit does not declare.
    pub(crate) fn rewrite(&self, value: &mut Value, schemas: &BTreeMap<String, Value>) {
        match value {
            Value::Object(map) => {
                if let Some(Value::String(reference)) = map.get_mut("$ref")
                    && let Some(rewritten) = self.external_ref(reference, schemas)
                {
                    *reference = rewritten;
                }
                for nested in map.values_mut() {
                    self.rewrite(nested, schemas);
                }
            }
            Value::Array(items) => {
                for item in items {
                    self.rewrite(item, schemas);
                }
            }
            _ => {}
        }
    }

    /// Document reference for a local `$ref` that names an imported unit.
    fn external_ref(&self, reference: &str, schemas: &BTreeMap<String, Value>) -> Option<String> {
        let name = reference.strip_prefix("#/components/schemas/")?;
        if schemas.contains_key(name) {
            return None;
        }
        let (prefix, rest) = name.split_once('.')?;
        if !self.prefixes.contains(prefix) {
            return None;
        }
        let (version, path) = match rest.split_once('.') {
            Some((version, path)) => (Some(version), format!("{version}.{path}")),
            None => (None, rest.to_string()),
        };
        let document = match version {
            Some(version) => format!("openrpc_{prefix}.{version}.json"),
            None => format!("openrpc_{prefix}.json"),
        };
        Some(format!("{document}#/components/schemas/{path}"))
    }
}
