//! Documents an OpenAPI document reaches through an import.

use std::collections::BTreeSet;

use xidl_parser::hir;
use xidl_parser::hir::ParserProperties;

/// Prefixes of the units a document imports.
///
/// Each unit is generated as its own document, so a reference into one is a
/// `$ref` to that document instead of a `$ref` inside the current one.
#[derive(Debug, Default, Clone)]
pub(crate) struct DocumentUnits {
    prefixes: BTreeSet<String>,
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
                    .filter_map(|import| import.get("prefix"))
                    .filter_map(serde_json::Value::as_str)
                    .map(str::to_string)
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
        if rest.is_empty() || name.is_root || !self.prefixes.contains(prefix) {
            return None;
        }
        let (document, path) = match rest.len() {
            1 => (format!("openapi_{prefix}.json"), rest[0].clone()),
            _ => (format!("openapi_{prefix}.{}.json", rest[0]), rest.join(".")),
        };
        Some(format!("{document}#/components/schemas/{path}"))
    }
}
