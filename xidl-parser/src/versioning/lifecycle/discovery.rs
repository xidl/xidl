//! Discovery of the `@version` enum and lifecycle validation.

use super::super::VersioningError;
use super::super::timeline::VersionTimeline;
use super::availability::{Availability, Site, VersionVisitor, availability, raw_versioning_name};
use crate::error::ParserResult;
use crate::hir::{Annotation, Definition, Enumerator};

/// Version enum found during discovery.
pub(crate) struct Discovery {
    /// Ident of the `@version` enum, when one is declared.
    pub(crate) version_enum: Option<(String, Vec<Enumerator>)>,
    /// First `@added`/`@removed` found outside a versioned specification.
    pub(crate) saw_lifecycle: Option<(String, String)>,
}

struct DiscoverVisitor {
    discovery: Discovery,
}

impl VersionVisitor for DiscoverVisitor {
    fn enter(&mut self, site: &Site<'_>, _parent: Availability) -> ParserResult<Availability> {
        site.validate_supported_annotations()?;
        let mut marker = false;
        let mut lifecycle: Vec<String> = Vec::new();
        for annotation in site.annotations {
            match annotation {
                Annotation::Version => marker = true,
                Annotation::Added { .. } => lifecycle.push("@added".to_string()),
                Annotation::Removed { .. } => lifecycle.push("@removed".to_string()),
                other => {
                    // A malformed `@version(...)` stays a raw application; treat
                    // it as the marker so its arguments are reported.
                    if raw_versioning_name(other).is_some_and(|name| {
                        crate::lint::AnnotationKind::from_name(&name)
                            == Some(crate::lint::AnnotationKind::Version)
                    }) {
                        marker = true;
                    }
                }
            }
        }
        if !marker {
            if let Some(annotation) = lifecycle.first() {
                if self.discovery.saw_lifecycle.is_none() {
                    self.discovery.saw_lifecycle = Some((annotation.clone(), site.path.clone()));
                }
            }
            return Ok(Availability::full());
        }

        let Some(members) = site.enum_members else {
            return Err(VersioningError::VersionOutsideEnum {
                target: format!("{} ({})", site.path, site.kind.describe()),
            }
            .into());
        };
        if let Some(annotation) = lifecycle.first() {
            return Err(VersioningError::VersionedVersionEnum {
                annotation: annotation.clone(),
                ident: site.path.clone(),
            }
            .into());
        }
        for member in members {
            for annotation in &member.annotations {
                if matches!(
                    annotation,
                    Annotation::Added { .. } | Annotation::Removed { .. }
                ) {
                    return Err(VersioningError::VersionedVersionEnum {
                        annotation: "@added/@removed".to_string(),
                        ident: format!("{}::{}", site.path, member.ident),
                    }
                    .into());
                }
            }
        }
        if let Some((first, _)) = &self.discovery.version_enum {
            return Err(VersioningError::DuplicateVersionEnum {
                first: first.clone(),
                second: site.path.clone(),
            }
            .into());
        }
        self.discovery.version_enum = Some((site.path.clone(), members.to_vec()));
        Ok(Availability::full())
    }
}

/// Finds the `@version` enum and reports `@version` misuse.
pub(crate) fn discover(definitions: &[Definition]) -> ParserResult<Discovery> {
    let mut visitor = DiscoverVisitor {
        discovery: Discovery {
            version_enum: None,
            saw_lifecycle: None,
        },
    };
    visitor.walk(definitions, "", Availability::full())?;
    Ok(visitor.discovery)
}

/// Validates every lifecycle annotation of a versioned specification.
pub(crate) struct LifecycleValidator<'a> {
    pub(crate) timeline: &'a VersionTimeline,
}

impl VersionVisitor for LifecycleValidator<'_> {
    fn enter(&mut self, site: &Site<'_>, parent: Availability) -> ParserResult<Availability> {
        availability(site, parent, self.timeline)
    }
}
