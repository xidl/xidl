//! Declaration sites, availability intervals, and visitor traits for versioning.

use super::super::timeline::VersionId;
use super::super::{VersioningDiagnostic, VersioningError};
use super::availability::{UNSUPPORTED_ANNOTATIONS, raw_versioning_name};
use crate::error::ParserResult;
use crate::hir::{Annotation, Enumerator};

/// Half-open interval `[from, until)` over the timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Availability {
    pub(crate) from: usize,
    pub(crate) until: usize,
}

impl Availability {
    pub(crate) fn full() -> Self {
        Self {
            from: 0,
            until: usize::MAX,
        }
    }

    pub(crate) fn contains(&self, version: VersionId) -> bool {
        version.index() >= self.from && version.index() < self.until
    }
}

/// Kind of source declaration visited by the lifecycle walker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SiteKind {
    Module,
    Type,
    Member,
    Enumerator,
    BitmaskValue,
    Bitfield,
    Interface,
    Operation,
    Parameter,
    Attribute,
    /// Annotatable declarations that `@added`/`@removed` cannot target.
    Unsupported,
}

impl SiteKind {
    pub(super) fn supports_lifecycle(self) -> bool {
        !matches!(self, Self::Unsupported | Self::Bitfield)
    }

    pub(super) fn describe(self) -> &'static str {
        match self {
            Self::Module => "module",
            Self::Type => "type declaration",
            Self::Member => "member",
            Self::Enumerator => "enumerator",
            Self::BitmaskValue => "bitmask value",
            Self::Bitfield => "bitfield",
            Self::Interface => "interface",
            Self::Operation => "operation",
            Self::Parameter => "parameter",
            Self::Attribute => "attribute",
            Self::Unsupported => "declaration",
        }
    }
}

/// One source declaration and its available annotation slot.
#[derive(Debug)]
pub(crate) struct Site<'a> {
    /// Declaration path, e.g. `PetStore::Pet::age`.
    pub(crate) path: String,
    /// Path of the enclosing declaration, or an empty string for the spec root.
    pub(crate) parent_path: String,
    pub(crate) kind: SiteKind,
    pub(crate) annotations: &'a [Annotation],
    /// Members of an enum declaration; `None` for every other site.
    pub(crate) enum_members: Option<&'a [Enumerator]>,
    /// Original parameter node, used to resolve its canonical HTTP binding.
    pub(crate) parameter: Option<&'a crate::hir::ParamDcl>,
}

impl<'a> Site<'a> {
    /// Collects every unsupported versioning annotation on this site.
    pub(crate) fn validate_supported_annotations_into(
        &self,
        errors: &mut Vec<VersioningDiagnostic>,
    ) {
        for annotation in self.annotations {
            if let Some(name) = raw_versioning_name(annotation) {
                if UNSUPPORTED_ANNOTATIONS.contains(&name.to_ascii_lowercase().as_str()) {
                    errors.push(VersioningDiagnostic {
                        location: self.path.clone(),
                        error: VersioningError::UnsupportedVersionAnnotation {
                            name: format!("@{name}"),
                            decl: self.path.clone(),
                        },
                    });
                }
            }
        }
    }

    /// Builds a site that carries no enum members.
    pub(crate) fn new(
        kind: SiteKind,
        path: &str,
        parent_path: &str,
        annotations: &'a [Annotation],
    ) -> Self {
        Self {
            path: path.to_string(),
            parent_path: parent_path.to_string(),
            kind,
            annotations,
            enum_members: None,
            parameter: None,
        }
    }

    /// Attaches the original parameter without reconstructing it from annotations.
    pub(crate) fn with_parameter(mut self, parameter: &'a crate::hir::ParamDcl) -> Self {
        self.parameter = Some(parameter);
        self
    }

    /// Attaches the members of an enum declaration.
    pub(crate) fn with_enum_members(mut self, members: &'a [Enumerator]) -> Self {
        self.enum_members = Some(members);
        self
    }
}

/// Callback invoked for every site while walking a specification.
pub(crate) trait VersionVisitor {
    /// Returns the availability that applies to the site's descendants.
    fn enter(&mut self, site: &Site<'_>, parent: Availability) -> ParserResult<Availability>;

    /// Walks every annotation-carrying site of a specification in source order.
    fn walk(
        &mut self,
        definitions: &[crate::hir::Definition],
        path: &str,
        parent: Availability,
    ) -> ParserResult<()>
    where
        Self: Sized,
    {
        super::walk::walk(definitions, path, parent, self)
    }
}
