use crate::generate::rust::util::{rust_ident, rust_scoped_name};
use xidl_parser::hir;

pub(super) fn canonical_name(module_path: &[String], ident: &str) -> String {
    if module_path.is_empty() {
        ident.to_string()
    } else {
        format!("{}::{}", module_path.join("::"), ident)
    }
}

pub(super) fn scoped_key(value: &hir::ScopedName) -> String {
    value.name.join("::")
}

pub(super) fn transport_ident(value: &str) -> String {
    value
        .split("::")
        .map(|part| part.to_string())
        .collect::<Vec<_>>()
        .join("_")
}

pub(super) fn transport_module(direction: &str, interface_ident: &str) -> String {
    format!("__xidl_{direction}_{interface_ident}")
}

pub(super) fn public_path_from_canonical(value: &str, module_path: &[String]) -> String {
    let parts = value.split("::").map(rust_ident).collect::<Vec<_>>();
    let current = module_path
        .iter()
        .map(|part| rust_ident(part))
        .collect::<Vec<_>>();
    if parts.starts_with(&current) {
        let suffix = parts[current.len()..].join("::");
        format!("super::{suffix}")
    } else {
        format!("crate::{}", parts.join("::"))
    }
}

pub(super) fn render_public_scoped(value: &hir::ScopedName) -> String {
    rust_scoped_name(value)
}
