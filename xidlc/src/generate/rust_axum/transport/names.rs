pub(super) fn canonical_name(module_path: &[String], ident: &str) -> String {
    if module_path.is_empty() {
        ident.to_string()
    } else {
        format!("{}::{}", module_path.join("::"), ident)
    }
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
