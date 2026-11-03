//! Published declaration excerpts from compiler-owned versioned HIR.

mod selection;
#[cfg(test)]
mod tests;

use crate::error::{DocumentationError, Result};
use selection::DeclarationSelection;
use std::collections::BTreeMap;
use xidl_parser::hir::{self, VisitScopedNames, printer::IdlPrinter};
use xidl_parser::rest_hir::{HttpOperationSource, RestHirDocument};

/// Canonical source declarations and HTTP operation-to-source identities.
pub(crate) struct IdlSource {
    spec: hir::Specification,
    operations: BTreeMap<String, String>,
    names: hir::DeclarationNames,
}

impl IdlSource {
    /// Uses an already projected specification, removing only its known version wrapper.
    pub fn new(document: &RestHirDocument, version: Option<&str>) -> Result<Self> {
        let mut spec = document.spec.clone();
        if let Some(version) = version {
            spec.0 = spec
                .0
                .into_iter()
                .flat_map(|definition| match definition {
                    hir::Definition::ModuleDcl(module) if module.ident == version => {
                        module.definition
                    }
                    hir::Definition::Pragma(_) => Vec::new(),
                    definition => vec![definition],
                })
                .collect();
            spec.0.remove_root_namespace(version);
        }
        let mut operations = BTreeMap::new();
        for interface in &document.interfaces {
            let mut scope = interface.module_path.clone();
            if version.is_some_and(|version| scope.first().is_some_and(|first| first == version)) {
                scope.remove(0);
            }
            scope.push(interface.name.clone());
            for operation in &interface.operations {
                let source = match operation.meta.source {
                    HttpOperationSource::Method => operation.meta.name.as_str(),
                    HttpOperationSource::AttributeGet => {
                        Self::attribute(&operation.meta.name, "get_attribute_")?
                    }
                    HttpOperationSource::AttributeSet => {
                        Self::attribute(&operation.meta.name, "set_attribute_")?
                    }
                    HttpOperationSource::AttributeWatch => {
                        Self::attribute(&operation.meta.name, "watch_attribute_")?
                    }
                };
                let prefix = scope.join("::");
                operations.insert(
                    format!("{prefix}::{}", operation.meta.name),
                    format!("{prefix}::{source}"),
                );
            }
        }
        let names = hir::DeclarationNames::collect(&spec.0);
        Ok(Self {
            spec,
            operations,
            names,
        })
    }

    /// Includes unpublished declarations so documentation keeps lexical shadowing.
    pub fn declares(&self, name: &str) -> bool {
        let path = name.split("::").collect::<Vec<_>>();
        self.names.contains(name)
            || self.operations.contains_key(name)
            || DeclarationSelection { path: &path }
                .find(&self.spec.0)
                .is_some()
    }

    /// Renders the actual owning declaration, retaining module and interface context.
    pub fn declaration(&self, name: &str) -> Option<String> {
        let source = self.operations.get(name).map_or(name, String::as_str);
        let path = source.split("::").collect::<Vec<_>>();
        DeclarationSelection { path: &path }
            .find(&self.spec.0)
            .map(|definition| IdlPrinter::render(&hir::Specification(vec![definition])))
    }

    fn attribute<'a>(name: &'a str, prefix: &str) -> Result<&'a str> {
        name.strip_prefix(prefix).ok_or_else(|| {
            DocumentationError::Contract(format!(
                "HTTP attribute operation '{name}' has no source declaration identity"
            ))
        })
    }
}
