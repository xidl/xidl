pub mod doc;
pub mod filter;
pub mod scope;

pub use doc::doc_lines_from_annotations;
pub use filter::rust_format_filter;

use xidl_parser::hir;

/// Whether any annotation carries the given name (e.g. `http`, `header`).
pub fn has_annotation(annotations: &[hir::Annotation], target: &str) -> bool {
    annotations.iter().any(|annotation| match annotation {
        hir::Annotation::Builtin { name, .. } => name == target,
        hir::Annotation::ScopedName { name, .. } => {
            name.name.last().map(String::as_str) == Some(target)
        }
        _ => false,
    })
}

use convert_case::{Case, Casing};

pub fn go_package_name(value: &str) -> String {
    let mut out = value.to_case(Case::Snake);
    out = out.replace('-', "_");
    if out.is_empty() {
        "xidl".to_string()
    } else {
        out
    }
}
