//! Source declaration facts carried between the unit graph and HIR projection.

use super::declarations::IntegerIndex;
use crate::error::{ParseError, ParserResult};
use crate::hir;
use crate::versioning::VersionDef;
use std::collections::{BTreeMap, BTreeSet};

/// Generator property containing immutable imported integer declarations.
pub const INTEGER_IMPORTS_PROPERTY: &str = "xidl_integer_imports";

/// The source declarations a unit exposes at each of its API versions.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct IntegerExports {
    versions: BTreeMap<String, IntegerIndex>,
    aliases: BTreeMap<String, String>,
    unversioned: Option<IntegerIndex>,
}

impl IntegerExports {
    /// Records a projected source contract without evaluating its constants.
    pub fn record(&mut self, spec: &hir::Specification, version: Option<&VersionDef>) {
        let index = IntegerIndex::collect(spec);
        if let Some(version) = version {
            self.versions.insert(version.ident.clone(), index);
            self.aliases
                .insert(version.value.clone(), version.ident.clone());
        } else {
            self.unversioned = Some(index);
        }
    }
}

/// Imported source contracts and the bindings visible to the current unit.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct IntegerImports {
    bindings: BTreeSet<String>,
    units: BTreeMap<String, ImportedUnit>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ImportedUnit {
    exports: IntegerExports,
    bindings: BTreeSet<String>,
}

impl IntegerImports {
    /// Starts a context containing only the current unit's declared imports.
    pub fn new(bindings: impl IntoIterator<Item = String>) -> Self {
        Self {
            bindings: bindings.into_iter().collect(),
            units: BTreeMap::new(),
        }
    }

    /// Adds one dependency's contracts together with its own import bindings.
    pub fn bind(
        &mut self,
        prefix: &str,
        exports: &IntegerExports,
        bindings: impl IntoIterator<Item = String>,
    ) {
        self.units.insert(
            prefix.to_owned(),
            ImportedUnit {
                exports: exports.clone(),
                bindings: bindings.into_iter().collect(),
            },
        );
    }

    /// Reads the typed source facts passed through the HIR generation protocol.
    pub fn from_properties(properties: &hir::ParserProperties) -> ParserResult<Self> {
        let Some(value) = properties.get(INTEGER_IMPORTS_PROPERTY) else {
            return Ok(Self::default());
        };
        serde_json::from_value(value.clone()).map_err(|error| {
            ParseError::Message(format!("integer import declarations metadata: {error}"))
        })
    }

    /// Finds a dependency contract at its canonical version.
    pub(super) fn index(&self, origin: &IntegerOrigin) -> Option<&IntegerIndex> {
        let IntegerOrigin::Imported { unit, version } = origin else {
            return None;
        };
        let exports = &self.units.get(unit)?.exports;
        match version {
            Some(version) => exports.versions.get(version),
            None => exports.unversioned.as_ref(),
        }
    }

    /// Resolves a reference only through bindings declared by its owning unit.
    pub(super) fn resolve(
        &self,
        origin: &IntegerOrigin,
        name: &hir::ScopedName,
    ) -> Option<IntegerScope> {
        let prefix = name.name.first()?;
        let bindings = match origin {
            IntegerOrigin::Local => &self.bindings,
            IntegerOrigin::Imported { unit, .. } => &self.units.get(unit)?.bindings,
        };
        if !bindings.contains(prefix) {
            return None;
        }
        let exports = &self.units.get(prefix)?.exports;
        let mut path = name.name.get(1..)?.to_vec();
        let (version, index) = if let Some(index) = &exports.unversioned {
            (None, index)
        } else {
            let written = path.first()?;
            let version = exports.aliases.get(written).unwrap_or(written).clone();
            let index = exports.versions.get(&version)?;
            path[0] = version.clone();
            (Some(version), index)
        };
        let resolved = index.names.resolve(
            &[],
            &hir::ScopedName {
                is_root: true,
                name: path,
            },
        )?;
        Some(IntegerScope {
            origin: IntegerOrigin::Imported {
                unit: prefix.clone(),
                version,
            },
            path: resolved.name,
        })
    }
}

/// The source unit and canonical version that own a declaration.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum IntegerOrigin {
    Local,
    Imported {
        unit: String,
        version: Option<String>,
    },
}

/// A lexical path retains its owner while resolving local and root references.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct IntegerScope {
    pub(super) origin: IntegerOrigin,
    pub(super) path: Vec<String>,
}

impl IntegerScope {
    /// Starts a lexical scope in the contract currently being projected.
    pub(super) fn local(path: &[String]) -> Self {
        Self {
            origin: IntegerOrigin::Local,
            path: path.to_vec(),
        }
    }

    /// Returns the lexical scope that contains this declaration.
    pub(super) fn parent(&self) -> Self {
        let mut parent = self.clone();
        parent.path.pop();
        parent
    }

    /// Names the owning unit, canonical version and complete declaration path.
    pub(super) fn qualified(&self) -> String {
        let unit = match &self.origin {
            IntegerOrigin::Local => None,
            IntegerOrigin::Imported { unit, .. } => Some(unit),
        };
        unit.into_iter()
            .chain(self.path.iter())
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join("::")
    }
}
