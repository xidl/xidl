pub mod doc;
#[cfg(feature = "gen-rust")]
pub mod filter;
pub mod scope;

pub use doc::doc_lines_from_annotations;
#[cfg(feature = "gen-rust")]
pub use filter::rust_format_filter;

#[cfg(any(feature = "gen-rust", feature = "gen-openapi"))]
use xidl_parser::hir;

/// Imported units that `definitions` reference, by their prefixes.
///
/// The rendered definitions already spell every reference, so a prefix that
/// appears as `<prefix><separator>` is a unit the file reaches into. Targets
/// that can bring the unit into scope turn each prefix into their own import.
pub fn referenced_imports(
    properties: &xidl_parser::hir::ParserProperties,
    definitions: &[String],
    separator: &str,
) -> Vec<String> {
    let Some(imports) = properties
        .get(crate::units::IMPORT_PROPERTY)
        .and_then(serde_json::Value::as_array)
    else {
        return Vec::new();
    };
    imports
        .iter()
        .filter_map(|import| import.get("prefix"))
        .filter_map(serde_json::Value::as_str)
        .filter(|prefix| {
            let marker = format!("{prefix}{separator}");
            definitions
                .iter()
                .any(|definition| definition.contains(&marker))
        })
        .map(str::to_string)
        .collect()
}

/// Prefixes of every unit `properties` imports.
#[cfg(feature = "gen-typescript")]
pub fn imported_units(properties: &xidl_parser::hir::ParserProperties) -> Vec<String> {
    properties
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
        .unwrap_or_default()
}

/// Whether any annotation carries the given name (e.g. `http`, `header`).
#[cfg(any(feature = "gen-rust", feature = "gen-openapi"))]
pub fn has_annotation(annotations: &[hir::Annotation], target: &str) -> bool {
    annotations.iter().any(|annotation| match annotation {
        hir::Annotation::Builtin { name, .. } => name == target,
        hir::Annotation::ScopedName { name, .. } => {
            name.name.last().map(String::as_str) == Some(target)
        }
        _ => false,
    })
}

#[cfg(feature = "gen-go")]
use convert_case::{Case, Casing};

#[cfg(feature = "gen-go")]
pub fn go_package_name(value: &str) -> String {
    let mut out = value.to_case(Case::Snake);
    out = out.replace('-', "_");
    if out.is_empty() {
        "xidl".to_string()
    } else {
        out
    }
}
