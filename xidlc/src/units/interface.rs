//! The declarations one unit exposes to the units that import it.

use super::error::ImportError;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use xidl_parser::hir;
use xidl_parser::semantic::IntegerExports;
use xidl_parser::versioning::VersioningRecord;

/// What one compilation unit exposes to its importers.
///
/// A versioned unit exposes one set of names per API version, which is what a
/// reference such as `timestamp::v1::Instant` is checked against. An
/// unversioned unit exposes one set for every reference.
#[derive(Debug, Clone)]
pub struct UnitInterface {
    per_version: BTreeMap<String, hir::DeclarationNames>,
    aliases: BTreeMap<String, String>,
    unversioned: Option<hir::DeclarationNames>,
    members: BTreeSet<String>,
    integer_exports: IntegerExports,
}

impl UnitInterface {
    /// Reads the interface `source` exposes.
    ///
    /// # Errors
    ///
    /// Returns [`ImportError::Parse`] when the unit cannot be read as IDL, so a
    /// broken dependency is reported against the file that holds it.
    pub(super) fn of(
        source: &str,
        path: &Path,
        mounts: &BTreeMap<String, PathBuf>,
    ) -> Result<Self, ImportError> {
        let spec = Self::lower(source, path, mounts)?;
        let record = VersioningRecord::analyze(&spec).map_err(|err| Self::error(path, err))?;
        let mut integer_exports = IntegerExports::default();
        let mut members = BTreeSet::new();
        let Some(record) = record else {
            integer_exports.record(&spec, None);
            Self::collect_members(&spec, &mut members);
            return Ok(Self {
                per_version: BTreeMap::new(),
                aliases: BTreeMap::new(),
                unversioned: Some(hir::DeclarationNames::collect(&spec.0)),
                members,
                integer_exports,
            });
        };
        let mut per_version = BTreeMap::new();
        let mut aliases = BTreeMap::new();
        for (id, definition) in record.timeline().iter() {
            let projected = record
                .project_source(&spec, id)
                .map_err(|err| Self::error(path, err))?;
            integer_exports.record(&projected, Some(definition));
            Self::collect_members(&projected, &mut members);
            per_version.insert(
                definition.ident.clone(),
                hir::DeclarationNames::collect(&projected.0),
            );
            aliases.insert(definition.value.clone(), definition.ident.clone());
        }
        Ok(Self {
            per_version,
            aliases,
            unversioned: None,
            members,
            integer_exports,
        })
    }

    /// Checks one reference against the interface of the unit it names.
    pub(super) fn check(
        &self,
        unit: &super::spec::Unit,
        target: &super::spec::Unit,
        reference: &super::refs::UnitRef,
    ) -> Result<(), ImportError> {
        let importer = unit.file().to_path_buf();
        let referenced = target.file().to_path_buf();
        let written = reference.qualified();
        let declared = |version: Option<&str>| self.declares(version, &reference.path());
        match reference.version() {
            Some(scope)
                if !self.is_versioned()
                    && self.declares(None, &format!("{scope}::{}", reference.path())) =>
            {
                Ok(())
            }
            Some(version) if !self.is_versioned() => Err(ImportError::UnexpectedVersion {
                importer,
                unit: referenced,
                reference: written,
                version: version.to_string(),
            }),
            Some(version) if !self.knows_version(version) => Err(ImportError::UnknownVersion {
                importer,
                unit: referenced,
                reference: written,
                version: version.to_string(),
                versions: self.versions(),
            }),
            Some(version) if !declared(Some(version)) => Err(ImportError::UnknownType {
                importer,
                unit: referenced,
                reference: written,
                version: Some(version.to_string()),
            }),
            Some(_) => Ok(()),
            None if self.is_versioned() => Err(ImportError::MissingVersion {
                importer,
                unit: referenced,
                reference: written,
                versions: self.versions(),
            }),
            None if !declared(None) => Err(ImportError::UnknownType {
                importer,
                unit: referenced,
                reference: written,
                version: None,
            }),
            None => Ok(()),
        }
    }

    /// Source declarations remain unevaluated until a consumer selects a version.
    pub(super) fn integer_exports(&self) -> &IntegerExports {
        &self.integer_exports
    }

    /// Whether the unit declares an API version timeline.
    pub(super) fn is_versioned(&self) -> bool {
        !self.per_version.is_empty()
    }

    /// Version idents the unit declares, in declaration order.
    pub(super) fn versions(&self) -> Vec<String> {
        self.per_version.keys().cloned().collect()
    }

    /// Whether `version` names a version of this unit, by ident or by value.
    pub(super) fn knows_version(&self, version: &str) -> bool {
        let ident = self
            .aliases
            .get(version)
            .map(String::as_str)
            .unwrap_or(version);
        self.per_version.contains_key(ident)
    }

    /// Whether `name` is declared at `version`.
    ///
    /// `version` may be a member ident or its display value.
    pub(super) fn declares(&self, version: Option<&str>, name: &str) -> bool {
        let Some(version) = version else {
            return self
                .unversioned
                .as_ref()
                .is_some_and(|names| names.contains(name));
        };
        let ident = self
            .aliases
            .get(version)
            .map(String::as_str)
            .unwrap_or(version);
        self.per_version
            .get(ident)
            .is_some_and(|names| names.contains(&format!("{ident}::{name}")))
    }

    /// Canonical enum and bitmask members exposed to generator targets.
    pub(super) fn members(&self) -> &BTreeSet<String> {
        &self.members
    }

    fn collect_members(spec: &hir::Specification, members: &mut BTreeSet<String>) {
        hir::DeclarationNames::visit_declarations(&spec.0, "", &mut |name, kind| {
            if kind == hir::DeclarationKind::Member {
                members.insert(name.to_string());
            }
        });
    }

    /// Lowers one unit into the HIR its interface is read from.
    fn lower(
        source: &str,
        path: &Path,
        mounts: &BTreeMap<String, PathBuf>,
    ) -> Result<hir::Specification, ImportError> {
        let mut resolver = crate::library::include_resolver(mounts);
        let typed =
            xidl_parser::parser::parser_text_with_resolver(source, path.to_str(), &mut *resolver)
                .map_err(|err| Self::error(path, err))?;
        // An export index needs source declarations, before selected constants
        // or array dimensions are evaluated during compilation.
        hir::Specification::source_from_typed_ast_with_resolver(typed, path, &mut *resolver)
            .map_err(|err| Self::error(path, err))
    }

    /// Names the file a failed interface read came from.
    fn error(path: &Path, reason: impl std::fmt::Display) -> ImportError {
        ImportError::Parse {
            path: path.to_path_buf(),
            reason: reason.to_string(),
        }
    }
}
