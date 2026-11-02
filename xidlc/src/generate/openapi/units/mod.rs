//! Documents an OpenAPI document reaches through an import.

use std::collections::{BTreeMap, BTreeSet};

use xidl_parser::hir;
use xidl_parser::hir::ParserProperties;

#[cfg(test)]
mod tests;

/// Prefixes of the units a document imports.
///
/// Each unit is generated as its own document, so a reference into one is a
/// `$ref` to that document instead of a `$ref` inside the current one.
#[derive(Debug, Default, Clone)]
pub(crate) struct DocumentUnits {
    prefixes: BTreeMap<String, BTreeSet<String>>,
}

impl DocumentUnits {
    /// Reads the prefixes of the units `properties` imports.
    pub(crate) fn of(properties: &ParserProperties) -> Self {
        let prefixes = properties
            .get(crate::units::IMPORT_PROPERTY)
            .and_then(serde_json::Value::as_array)
            .map(|imports| {
                imports
                    .iter()
                    .filter_map(|import| {
                        let prefix = import.get("prefix")?.as_str()?.to_owned();
                        let versions = import
                            .get("versions")
                            .and_then(serde_json::Value::as_array)
                            .into_iter()
                            .flatten()
                            .filter_map(serde_json::Value::as_str)
                            .map(str::to_owned)
                            .collect();
                        Some((prefix, versions))
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self { prefixes }
    }

    /// Reference location `name` resolves to in another document.
    ///
    /// A versioned unit is generated once per version, and the projection keeps
    /// the version module in the schema name, so both the document and the
    /// schema carry the version.
    pub(crate) fn reference(&self, name: &hir::ScopedName) -> Option<String> {
        let (prefix, rest) = name.name.split_first()?;
        if rest.is_empty() || name.is_root {
            return None;
        }
        let versions = self.prefixes.get(prefix)?;
        let document = if versions.contains(&rest[0]) {
            format!("openapi_{prefix}.{}.json", rest[0])
        } else {
            format!("openapi_{prefix}.json")
        };
        let path = rest.join(".");
        Some(format!("{document}#/components/schemas/{path}"))
    }
}
