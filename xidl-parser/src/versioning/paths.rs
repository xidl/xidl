//! Declaration paths shared by the versioning walk and projection.

/// Joins a declaration path with a child identifier using `::`.
pub(crate) fn join(path: &str, ident: &str) -> String {
    if path.is_empty() {
        ident.to_string()
    } else {
        format!("{path}::{ident}")
    }
}

/// Name of one declarator, including array declarators.
pub(crate) fn declarator_name(declarator: &crate::hir::Declarator) -> String {
    match declarator {
        crate::hir::Declarator::SimpleDeclarator(simple) => simple.0.clone(),
        crate::hir::Declarator::ArrayDeclarator(array) => array.ident.clone(),
    }
}

/// Comma separated names of a declarator list.
pub(crate) fn declarators_name(declarators: &[crate::hir::Declarator]) -> String {
    declarators
        .iter()
        .map(declarator_name)
        .collect::<Vec<_>>()
        .join(", ")
}
