//! Published symbol destinations with IDL lexical scope and import bindings.

use crate::config::DocumentationConfig;
use crate::contract::Contract;
use crate::idl::IdlSource;
use crate::links::SchemaId;
use serde_json::Value;
use std::collections::BTreeMap;
use xidlc::jsonrpc::{CompilationUnit, UnitImport};

/// Unit, selected version and lexical declaration that own a Markdown fragment.
pub(crate) struct SymbolScope<'a> {
    pub unit: &'a str,
    pub version: &'a str,
    pub name: &'a str,
}

/// Published destinations resolved against compiler-owned declaration identities.
pub(crate) struct Symbols<'a> {
    publications: BTreeMap<(String, String), Publication<'a>>,
    imports: BTreeMap<&'a str, &'a [UnitImport]>,
}

struct Publication<'a> {
    source: &'a IdlSource,
    links: BTreeMap<String, String>,
}

impl<'a> Symbols<'a> {
    /// Indexes existing reference pages without publishing extra declarations.
    pub fn new(
        contracts: &'a [Contract],
        schemas: &BTreeMap<SchemaId, Value>,
        units: &'a [CompilationUnit],
        config: &DocumentationConfig,
    ) -> Self {
        let mut publications = BTreeMap::new();
        for contract in contracts {
            publications.insert(
                (contract.unit.clone(), contract.version.clone()),
                Publication {
                    source: &contract.idl,
                    links: contract
                        .operations
                        .keys()
                        .map(|name| (name.clone(), config.url(&contract.operation_path(name))))
                        .collect(),
                },
            );
        }
        for id in schemas.keys() {
            if let Some(publication) = publications.get_mut(&(id.unit.clone(), id.version.clone()))
            {
                publication
                    .links
                    .insert(id.logical_name(), config.url(&id.path()));
            }
        }
        Self {
            publications,
            imports: units
                .iter()
                .map(|unit| (unit.id.as_str(), unit.imports.as_slice()))
                .collect(),
        }
    }

    /// Resolves an IDL name without falling through a shadowing unpublished declaration.
    pub fn resolve(&self, scope: &SymbolScope<'_>, reference: &str) -> Option<&str> {
        let reference = reference
            .strip_prefix('`')
            .and_then(|reference| reference.strip_suffix('`'))
            .unwrap_or(reference);
        let rooted = reference.starts_with("::");
        let name = reference.strip_prefix("::").unwrap_or(reference);
        if !name.split("::").all(Self::identifier) {
            return None;
        }
        let publication = self
            .publications
            .get(&(scope.unit.into(), scope.version.into()))?;
        let mut parent = if rooted { "" } else { scope.name };
        loop {
            let candidate = if parent.is_empty() {
                name.to_owned()
            } else {
                format!("{parent}::{name}")
            };
            if let Some(url) = publication.links.get(&candidate) {
                return Some(url);
            }
            if publication.source.declares(&candidate) {
                return None;
            }
            if parent.is_empty() {
                break;
            }
            parent = parent.rsplit_once("::").map_or("", |(parent, _)| parent);
        }
        if rooted {
            return None;
        }
        self.imported(scope.unit, name)
    }

    fn imported(&self, unit: &str, name: &str) -> Option<&str> {
        let (prefix, name) = name.split_once("::")?;
        let import = self
            .imports
            .get(unit)?
            .iter()
            .find(|import| import.prefix == prefix)?;
        let (version, name) = match name.split_once("::") {
            Some((version, rest)) if import.versions.iter().any(|known| known == version) => {
                (version, rest)
            }
            _ => (import.version.as_deref().unwrap_or("unversioned"), name),
        };
        self.publications
            .get(&(prefix.into(), version.into()))?
            .links
            .get(name)
            .map(String::as_str)
    }

    fn identifier(name: &str) -> bool {
        let mut bytes = name.bytes();
        bytes
            .next()
            .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
            && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    }
}
