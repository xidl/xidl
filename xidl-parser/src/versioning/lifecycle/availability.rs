//! Availability intervals derived from `@added` and `@removed`.

use super::super::VersioningError;
use super::super::timeline::{VersionId, VersionTimeline};
use crate::error::ParserResult;
use crate::hir::{Annotation, Enumerator, ScopedName};

/// Annotations reserved for later versioning phases.
pub(super) const UNSUPPORTED_ANNOTATIONS: [&str; 7] = [
    "versioned",
    "renamedfrom",
    "madeoptional",
    "maderequired",
    "typechangedfrom",
    "returntypechangedfrom",
    "defaultversion",
];

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

/// Kind of annotation-carrying site visited by the walker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SiteKind {
    Module,
    Type,
    Member,
    Enumerator,
    BitmaskValue,
    Interface,
    Operation,
    Parameter,
    Attribute,
    /// Annotatable declarations that `@added`/`@removed` cannot target.
    Unsupported,
}

impl SiteKind {
    fn supports_lifecycle(self) -> bool {
        !matches!(self, Self::Unsupported)
    }

    pub(super) fn describe(self) -> &'static str {
        match self {
            Self::Module => "module",
            Self::Type => "type declaration",
            Self::Member => "member",
            Self::Enumerator => "enumerator",
            Self::BitmaskValue => "bitmask value",
            Self::Interface => "interface",
            Self::Operation => "operation",
            Self::Parameter => "parameter",
            Self::Attribute => "attribute",
            Self::Unsupported => "declaration",
        }
    }
}

/// One annotation-carrying site of a specification.
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
}

impl<'a> Site<'a> {
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
        }
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

/// Computes the availability of one site, validating its lifecycle annotations.
///
/// Annotation order never matters: `@added` and `@removed` are resolved against
/// the timeline and combined by index.
pub(crate) fn availability(
    site: &Site<'_>,
    parent: Availability,
    timeline: &VersionTimeline,
) -> ParserResult<Availability> {
    let mut added: Option<VersionId> = None;
    let mut removed: Option<VersionId> = None;
    let mut added_name = String::new();
    let mut removed_name = String::new();

    for annotation in site.annotations {
        match annotation {
            Annotation::Added { version } => {
                if added.is_some() {
                    return Err(VersioningError::DuplicateLifecycleAnnotation {
                        annotation: "@added".to_string(),
                        decl: site.path.clone(),
                    }
                    .into());
                }
                let resolved = resolve_reference(timeline, "@added", &site.path, version)?;
                added_name = render_reference(version);
                added = Some(resolved);
            }
            Annotation::Removed { version } => {
                if removed.is_some() {
                    return Err(VersioningError::DuplicateLifecycleAnnotation {
                        annotation: "@removed".to_string(),
                        decl: site.path.clone(),
                    }
                    .into());
                }
                let resolved = resolve_reference(timeline, "@removed", &site.path, version)?;
                removed_name = render_reference(version);
                removed = Some(resolved);
            }
            Annotation::Version => {}
            other => {
                let Some(name) = raw_versioning_name(other) else {
                    continue;
                };
                let lowered = name.to_ascii_lowercase();
                if lowered == "added" || lowered == "removed" {
                    return Err(VersioningError::MalformedVersionReference {
                        annotation: format!("@{name}"),
                        decl: site.path.clone(),
                    }
                    .into());
                }
                if lowered == "version" {
                    return Err(VersioningError::MalformedVersionMarker {
                        decl: site.path.clone(),
                    }
                    .into());
                }
                if UNSUPPORTED_ANNOTATIONS.contains(&lowered.as_str()) {
                    return Err(VersioningError::UnsupportedVersionAnnotation {
                        name: format!("@{name}"),
                        decl: site.path.clone(),
                    }
                    .into());
                }
            }
        }
    }

    if (added.is_some() || removed.is_some()) && !site.kind.supports_lifecycle() {
        let annotation = if added.is_some() {
            "@added"
        } else {
            "@removed"
        };
        return Err(VersioningError::UnsupportedVersionTarget {
            annotation: annotation.to_string(),
            target: format!("{} ({})", site.path, site.kind.describe()),
        }
        .into());
    }

    let own_from = added.map(VersionId::index).unwrap_or(0);
    let own_until = removed.map(VersionId::index).unwrap_or(timeline.len());
    if own_from >= own_until {
        return Err(VersioningError::EmptyAvailability {
            decl: site.path.clone(),
            added: added_name,
            removed: removed_name,
        }
        .into());
    }

    if let Some(version) = added {
        if version.index() < parent.from || version.index() >= parent.until {
            return Err(VersioningError::ChildOutsideParent {
                decl: site.path.clone(),
                parent: site.parent_path.clone(),
                version: added_name,
            }
            .into());
        }
    }
    if let Some(version) = removed {
        if version.index() >= parent.until || version.index() <= parent.from {
            return Err(VersioningError::ChildOutsideParent {
                decl: site.path.clone(),
                parent: site.parent_path.clone(),
                version: removed_name,
            }
            .into());
        }
    }

    Ok(Availability {
        from: own_from.max(parent.from),
        until: own_until.min(parent.until),
    })
}

/// Resolves an `Enum::member` version reference used by a lifecycle annotation.
pub(crate) fn resolve_reference(
    timeline: &VersionTimeline,
    annotation: &str,
    decl: &str,
    reference: &ScopedName,
) -> ParserResult<VersionId> {
    timeline.resolve(reference).ok_or_else(|| {
        VersioningError::UnknownVersionReference {
            annotation: annotation.to_string(),
            decl: decl.to_string(),
            reference: render_reference(reference),
            declared: timeline.declared(),
        }
        .into()
    })
}

/// Renders a scoped name the way it appears in IDL source.
pub(crate) fn render_reference(reference: &ScopedName) -> String {
    let prefix = if reference.is_root { "::" } else { "" };
    format!("{prefix}{}", reference.name.join("::"))
}

/// Final annotation name for name-based fallbacks, keeping the source spelling.
pub(super) fn raw_versioning_name(annotation: &Annotation) -> Option<String> {
    match annotation {
        Annotation::Builtin { name, .. } => Some(name.clone()),
        Annotation::ScopedName { name, .. } => name.name.last().cloned(),
        _ => None,
    }
}

/// Whether an annotation is `@version` or a lifecycle annotation.
pub(crate) fn is_versioning_annotation(annotation: &Annotation) -> bool {
    match annotation {
        Annotation::Version | Annotation::Added { .. } | Annotation::Removed { .. } => true,
        other => raw_versioning_name(other).is_some_and(|name| {
            name.eq_ignore_ascii_case("version")
                || name.eq_ignore_ascii_case("added")
                || name.eq_ignore_ascii_case("removed")
        }),
    }
}
