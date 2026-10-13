//! API versioning: one `@version` enum, `@added`/`@removed` lifecycle annotations
//! and per-version HIR projection.
//!
//! Phase 1 is deliberately small. A specification is versioned by marking one
//! enum with `@version`; its members, in declaration order, are the version
//! history. `@added(Versions::v1)` and `@removed(Versions::v2)` mark when a
//! declaration exists; annotation order never matters. Renames are expressed by
//! declaring a new member with a disjoint interval plus `@rename`, exactly like
//! the desugaring `versioned idl -> multi idl` describes.
//!
//! Projection is a pure HIR rewrite: each version is wrapped in
//! `module <version-ident> { ... }`, lifecycle annotations and the version enum
//! are removed, and the document version pragma carries the version value. The
//! merged form keeps one module per selected version so generators can stay
//! version-free.

mod error;
mod lifecycle;
mod paths;
mod project;
mod timeline;

#[cfg(test)]
mod tests;

pub use error::VersioningError;
pub(crate) use timeline::VersionSelection;
pub use timeline::{VersionDef, VersionId, VersionTimeline};

use lifecycle::VersionVisitor as _;

use crate::error::{ParseError, ParserResult};
use crate::hir::{ParserProperties, Specification};
use crate::rest_hir::ProjectedHir;

/// Generation property that selects one API version by member ident or value.
pub const API_VERSION_PROPERTY: &str = "api_version";
/// Generation property that selects several API versions (comma separated, or `*`).
pub const API_VERSIONS_PROPERTY: &str = "api_versions";
/// Generation property that selects the version output layout.
pub const VERSION_LAYOUT_PROPERTY: &str = "version_layout";
/// Generation property that lists the API version modules of a projection.
///
/// The HIR stage sets it for every versioned contract; targets that cannot
/// render module-scoped output yet use it to fail loudly instead of emitting
/// code that does not compile.
pub const API_VERSION_MODULES_PROPERTY: &str = "api_version_modules";
/// Layout value: one specification holding one module per version.
pub const LAYOUT_MERGED: &str = "merged";
/// Layout value: one projection per version.
pub const LAYOUT_PER_VERSION: &str = "per_version";

/// One HIR projection produced from a specification.
#[derive(Debug)]
pub struct VersionedProjection {
    /// API versions this projection contains; empty for unversioned contracts.
    pub versions: Vec<VersionDef>,
    /// Target version of a single-version projection; `None` for merged and
    /// unversioned projections.
    pub version: Option<VersionDef>,
    /// The projected HIR.
    pub hir: ProjectedHir,
}

/// Result of analysing the API versioning annotations of a specification.
#[derive(Debug, Clone)]
pub struct VersioningRecord {
    timeline: VersionTimeline,
}

impl VersioningRecord {
    /// Validates every API versioning annotation of a specification.
    ///
    /// Validation always runs, even when no version is projected. Returns
    /// `None` for specifications without a `@version` enum, and fails when
    /// lifecycle annotations appear without one.
    pub fn analyze(spec: &Specification) -> ParserResult<Option<VersioningRecord>> {
        let discovery = lifecycle::discover(&spec.0)?;
        let Some((ident, members)) = discovery.version_enum else {
            if let Some((annotation, decl)) = discovery.saw_lifecycle {
                return Err(VersioningError::MissingVersionEnum { annotation, decl }.into());
            }
            return Ok(None);
        };
        let timeline = VersionTimeline::from_members(ident, &members)?;
        let mut validator = lifecycle::LifecycleValidator {
            timeline: &timeline,
        };
        validator.walk(&spec.0, "", lifecycle::Availability::full())?;
        Ok(Some(VersioningRecord { timeline }))
    }

    /// The version history declared by the `@version` enum.
    pub fn timeline(&self) -> &VersionTimeline {
        &self.timeline
    }

    /// Projects `version` into an ordinary specification.
    pub fn project(&self, spec: &Specification, version: VersionId) -> ParserResult<Specification> {
        project::project_version(spec, &self.timeline, version)
    }

    /// Projects every selected version into one specification with one module
    /// per version, in timeline order.
    pub fn merge(
        &self,
        spec: &Specification,
        versions: &[VersionId],
    ) -> ParserResult<Specification> {
        project::merge_versions(spec, &self.timeline, versions)
    }
}

/// How versioned output is laid out for the target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VersionLayout {
    /// One specification with one module per selected version.
    Merged,
    /// One projection per selected version.
    PerVersion,
}

impl VersionLayout {
    /// Reads [`VERSION_LAYOUT_PROPERTY`] from generation properties.
    pub(crate) fn from_properties(properties: &ParserProperties) -> ParserResult<Self> {
        let Some(value) = properties
            .get(VERSION_LAYOUT_PROPERTY)
            .and_then(serde_json::Value::as_str)
        else {
            return Ok(Self::Merged);
        };
        match value {
            LAYOUT_MERGED => Ok(Self::Merged),
            LAYOUT_PER_VERSION => Ok(Self::PerVersion),
            other => Err(VersioningError::UnknownVersionLayout {
                value: other.to_string(),
            }
            .into()),
        }
    }
}

/// Reads the requested version selection from generation properties.
///
/// A selection property on a specification without a `@version` enum is an
/// error, so build flags cannot silently do nothing.
pub(crate) fn selection_from_properties(
    properties: &ParserProperties,
) -> ParserResult<Option<VersionSelection>> {
    VersionSelection::from_properties(properties)
}

/// Resolves the requested versions, defaulting to every declared version.
pub(crate) fn selected_versions(
    selection: Option<&VersionSelection>,
    record: &VersioningRecord,
) -> ParserResult<Vec<VersionId>> {
    match selection {
        Some(selection) => selection.resolve(&record.timeline),
        None => Ok(record.timeline.iter().map(|(id, _)| id).collect()),
    }
}

/// Reports a selection requested for an unversioned specification.
pub(crate) fn unversioned_selection_error(selection: &VersionSelection) -> ParseError {
    VersioningError::UnversionedSpecification {
        selection: selection.describe(),
    }
    .into()
}
