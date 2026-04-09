use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopedName {
    pub name: Vec<String>,
    pub is_root: bool,
}

impl From<crate::typed_ast::ScopedName> for ScopedName {
    fn from(value: crate::typed_ast::ScopedName) -> Self {
        let is_root = value.node_text.starts_with("::");
        let mut parts = Vec::new();
        collect(&mut parts, &value);
        Self {
            name: parts.into_iter().map(ToOwned::to_owned).collect(),
            is_root,
        }
    }
}

fn collect<'a>(parts: &mut Vec<&'a str>, value: &'a crate::typed_ast::ScopedName) {
    if let Some(parent) = &value.scoped_name {
        collect(parts, parent);
    }
    parts.push(&value.identifier.0);
}
