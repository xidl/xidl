//! Names used by ordinary Rust unions and their independent adapters.

use convert_case::{Case, Casing};

/// Rust variant identifier for a built-in HTTP content type.
pub fn http_union_case_ident(ident: &str) -> String {
    ident
        .strip_prefix("r#")
        .unwrap_or(ident)
        .to_case(Case::Pascal)
}

/// Wire discriminator spelling used by the ordinary union's Serde layout.
pub fn union_serde_tag_string(label: &str) -> String {
    label.rsplit("::").next().unwrap_or(label).to_string()
}
