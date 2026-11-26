//! Availability intervals derived from `@added` and `@removed`.

use super::super::timeline::{VersionId, VersionTimeline};
use super::super::{VersioningDiagnostic, VersioningError};
pub(crate) use super::site::{Availability, Site, SiteKind, VersionVisitor};
use crate::error::ParserResult;
use crate::hir::{Annotation, ScopedName};
use crate::lint::AnnotationKind;

/// Rejected versioning annotations, rather than promises of future syntax.
pub(super) const UNSUPPORTED_ANNOTATIONS: [&str; 8] = [
    "versioned",
    "renamedfrom",
    "replaces",
    "madeoptional",
    "maderequired",
    "typechangedfrom",
    "returntypechangedfrom",
    "defaultversion",
];

fn push_diag(errors: &mut Vec<VersioningDiagnostic>, location: &str, error: VersioningError) {
    errors.push(VersioningDiagnostic {
        location: location.to_string(),
        error,
    });
}

struct LifecycleSlot<'a> {
    label: &'static str,
    version: &'a mut Option<VersionId>,
    name: &'a mut String,
}

fn apply_lifecycle_ref(
    site: &Site<'_>,
    reference: &ScopedName,
    slot: LifecycleSlot<'_>,
    timeline: &VersionTimeline,
    errors: &mut Vec<VersioningDiagnostic>,
) -> bool {
    if slot.version.is_some() {
        push_diag(
            errors,
            &site.path,
            VersioningError::DuplicateLifecycleAnnotation {
                annotation: slot.label.to_string(),
                decl: site.path.clone(),
            },
        );
        return false;
    }
    match resolve_reference(timeline, slot.label, &site.path, reference) {
        Ok(resolved) => {
            *slot.name = render_reference(reference);
            *slot.version = Some(resolved);
            true
        }
        Err(crate::error::ParseError::Versioning(error)) => {
            push_diag(errors, &site.path, error);
            false
        }
        Err(_) => false,
    }
}

/// Computes availability for one site, recording every lifecycle violation into `errors`.
pub(crate) fn availability_into(
    site: &Site<'_>,
    parent: Availability,
    timeline: &VersionTimeline,
    errors: &mut Vec<VersioningDiagnostic>,
) -> Availability {
    let mut added: Option<VersionId> = None;
    let mut removed: Option<VersionId> = None;
    let mut added_name = String::new();
    let mut removed_name = String::new();
    let mut invalid = false;

    for annotation in site.annotations {
        match annotation {
            Annotation::Added { version } => {
                let slot = LifecycleSlot {
                    label: "@added",
                    version: &mut added,
                    name: &mut added_name,
                };
                invalid |= !apply_lifecycle_ref(site, version, slot, timeline, errors);
            }
            Annotation::Removed { version } => {
                let slot = LifecycleSlot {
                    label: "@removed",
                    version: &mut removed,
                    name: &mut removed_name,
                };
                invalid |= !apply_lifecycle_ref(site, version, slot, timeline, errors);
            }
            Annotation::Version | Annotation::VersionParameter(_) => {}
            other => {
                let Some(name) = raw_versioning_name(other) else {
                    continue;
                };
                match AnnotationKind::from_name(name.as_str()) {
                    Some(AnnotationKind::Added | AnnotationKind::Removed) => {
                        push_diag(
                            errors,
                            &site.path,
                            VersioningError::MalformedVersionReference {
                                annotation: format!("@{name}"),
                                decl: site.path.clone(),
                            },
                        );
                        invalid = true;
                    }
                    Some(AnnotationKind::Version) => {
                        push_diag(
                            errors,
                            &site.path,
                            VersioningError::MalformedVersionMarker {
                                decl: site.path.clone(),
                            },
                        );
                        invalid = true;
                    }
                    _ => {}
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
        push_diag(
            errors,
            &site.path,
            VersioningError::UnsupportedVersionTarget {
                annotation: annotation.to_string(),
                target: format!("{} ({})", site.path, site.kind.describe()),
            },
        );
        return parent;
    }
    if invalid {
        return parent;
    }

    let own_from = added.map(VersionId::index).unwrap_or(0);
    let own_until = removed.map(VersionId::index).unwrap_or(timeline.len());
    if own_from >= own_until {
        push_diag(
            errors,
            &site.path,
            VersioningError::EmptyAvailability {
                decl: site.path.clone(),
                added: added_name,
                removed: removed_name,
            },
        );
        return parent;
    }

    let mut out_of_bounds = false;
    for (version, name, outside) in [
        (
            added,
            added_name,
            added.is_some_and(|v| v.index() < parent.from || v.index() >= parent.until),
        ),
        (
            removed,
            removed_name,
            removed.is_some_and(|v| v.index() >= parent.until || v.index() <= parent.from),
        ),
    ] {
        if version.is_some() && outside {
            push_diag(
                errors,
                &site.path,
                VersioningError::ChildOutsideParent {
                    decl: site.path.clone(),
                    parent: site.parent_path.clone(),
                    version: name,
                },
            );
            out_of_bounds = true;
        }
    }
    if out_of_bounds {
        return parent;
    }

    Availability {
        from: own_from.max(parent.from),
        until: own_until.min(parent.until),
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
    let mut errors = Vec::new();
    site.validate_supported_annotations_into(&mut errors);
    if let Some(first) = errors.drain(..).next() {
        return Err(first.error.into());
    }
    let avail = availability_into(site, parent, timeline, &mut errors);
    if let Some(first) = errors.drain(..).next() {
        return Err(first.error.into());
    }
    Ok(avail)
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
        Annotation::Version
        | Annotation::VersionParameter(_)
        | Annotation::Added { .. }
        | Annotation::Removed { .. } => true,
        other => raw_versioning_name(other).is_some_and(|name| {
            AnnotationKind::from_name(&name).is_some_and(AnnotationKind::is_lifecycle)
        }),
    }
}
