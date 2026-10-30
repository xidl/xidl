//! Lexical IDL lookup composed with Go package qualification.

use xidl_parser::hir;

use super::definition::go_capitalize;
use super::{GoNames, GoPackages};

/// The declaration scope and package imports for one Go rendering site.
#[derive(Clone, Copy)]
pub(crate) struct GoScope<'a> {
    names: &'a GoNames,
    pub(crate) packages: &'a GoPackages,
    path: &'a [String],
}

impl<'a> GoScope<'a> {
    /// Combines declaration names, imports and the current IDL path.
    pub(crate) fn new(names: &'a GoNames, packages: &'a GoPackages, path: &'a [String]) -> Self {
        Self {
            names,
            packages,
            path,
        }
    }

    /// Renders a declaration reference, qualifying imported units as needed.
    pub(crate) fn reference(self, name: &hir::ScopedName) -> String {
        if let Some(resolved) = self.names.resolve(self.path, name) {
            return resolved.to_string();
        }
        self.packages
            .reference(name)
            .unwrap_or_else(|| name.name.iter().map(|part| go_capitalize(part)).collect())
    }

    /// Renders a type with the same scope throughout its nested containers.
    pub(crate) fn type_ref(self, ty: &hir::TypeSpec) -> String {
        super::definition::go_type(ty, self)
    }
}
