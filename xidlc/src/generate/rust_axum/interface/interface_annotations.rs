pub(crate) fn serde_rename(wire_name: &str, rust_name: &str) -> Option<String> {
    if wire_name != rust_name {
        Some(wire_name.to_string())
    } else {
        None
    }
}
