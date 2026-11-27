//! Discovery of the `@version` enum and lifecycle validation.

use super::super::timeline::VersionTimeline;
use super::super::{VersioningDiagnostic, VersioningError};
use super::availability::{
    Availability, Site, VersionVisitor, availability_into, raw_versioning_name,
};
use crate::error::ParserResult;
use crate::hir::{Annotation, Definition, Enumerator, VersionParameter};

/// Version enum found during discovery.
pub(crate) struct Discovery {
    /// Ident of the `@version` enum, when one is declared.
    pub(crate) version_enum: Option<(String, Vec<Enumerator>)>,
    /// All `@added`/`@removed` sites found outside a versioned specification.
    pub(crate) saw_lifecycle: Vec<(String, String)>,
    /// The request parameter declared by the version timeline.
    pub(crate) version_parameter: Option<VersionParameter>,
}

struct DiscoverVisitor {
    discovery: Discovery,
    errors: Vec<VersioningDiagnostic>,
}

impl DiscoverVisitor {
    fn push(&mut self, location: &str, error: VersioningError) {
        self.errors.push(VersioningDiagnostic {
            location: location.to_string(),
            error,
        });
    }
}

impl VersionVisitor for DiscoverVisitor {
    fn enter(&mut self, site: &Site<'_>, _parent: Availability) -> ParserResult<Availability> {
        site.validate_supported_annotations_into(&mut self.errors);
        let mut marker = false;
        let mut lifecycle: Vec<String> = Vec::new();
        let mut parameter: Option<VersionParameter> = None;
        for annotation in site.annotations {
            match annotation {
                Annotation::Version => {
                    if marker {
                        self.push(
                            &site.path,
                            VersioningError::DuplicateVersionParameter {
                                decl: site.path.clone(),
                            },
                        );
                    } else {
                        marker = true;
                    }
                }
                Annotation::VersionParameter(value) => {
                    if marker {
                        self.push(
                            &site.path,
                            VersioningError::DuplicateVersionParameter {
                                decl: site.path.clone(),
                            },
                        );
                        continue;
                    }
                    marker = true;
                    match value.validate() {
                        Ok(()) => parameter = Some(value.clone()),
                        Err(reason) => self.push(
                            &site.path,
                            VersioningError::MalformedVersionParameter {
                                decl: site.path.clone(),
                                reason,
                            },
                        ),
                    }
                }
                Annotation::Added { .. } => lifecycle.push("@added".to_string()),
                Annotation::Removed { .. } => lifecycle.push("@removed".to_string()),
                other => {
                    // A malformed `@version(...)` stays a raw application; treat
                    // it as the marker so its arguments are reported.
                    if raw_versioning_name(other).is_some_and(|name| {
                        crate::annotations::AnnotationKind::from_name(&name)
                            == Some(crate::annotations::AnnotationKind::Version)
                    }) {
                        if marker {
                            self.push(
                                &site.path,
                                VersioningError::DuplicateVersionParameter {
                                    decl: site.path.clone(),
                                },
                            );
                            continue;
                        }
                        marker = true;
                        match crate::hir::annotation_params(other) {
                            None => self.push(
                                &site.path,
                                VersioningError::MalformedVersionParameter {
                                    decl: site.path.clone(),
                                    reason: "requires in=... and name=...".to_string(),
                                },
                            ),
                            Some(value) => match VersionParameter::parse(value) {
                                Ok(parsed) => parameter = Some(parsed),
                                Err(reason) => self.push(
                                    &site.path,
                                    VersioningError::MalformedVersionParameter {
                                        decl: site.path.clone(),
                                        reason,
                                    },
                                ),
                            },
                        }
                    }
                }
            }
        }
        if !marker {
            if let Some(annotation) = lifecycle.first() {
                self.discovery
                    .saw_lifecycle
                    .push((annotation.clone(), site.path.clone()));
            }
            return Ok(Availability::full());
        }

        let Some(members) = site.enum_members else {
            self.push(
                &site.path,
                VersioningError::VersionOutsideEnum {
                    target: format!("{} ({})", site.path, site.kind.describe()),
                },
            );
            return Ok(Availability::full());
        };
        if let Some(annotation) = lifecycle.first() {
            self.push(
                &site.path,
                VersioningError::VersionedVersionEnum {
                    annotation: annotation.clone(),
                    ident: site.path.clone(),
                },
            );
        }
        for member in members {
            for annotation in &member.annotations {
                if matches!(
                    annotation,
                    Annotation::Added { .. } | Annotation::Removed { .. }
                ) {
                    let member_path = format!("{}::{}", site.path, member.ident);
                    self.push(
                        &member_path,
                        VersioningError::VersionedVersionEnum {
                            annotation: "@added/@removed".to_string(),
                            ident: member_path.clone(),
                        },
                    );
                }
            }
        }
        if let Some((first, _)) = &self.discovery.version_enum {
            self.push(
                &site.path,
                VersioningError::DuplicateVersionEnum {
                    first: first.clone(),
                    second: site.path.clone(),
                },
            );
            return Ok(Availability::full());
        }
        self.discovery.version_enum = Some((site.path.clone(), members.to_vec()));
        self.discovery.version_parameter = parameter;
        Ok(Availability::full())
    }
}

/// Finds the `@version` enum and collects every `@version` misuse.
pub(crate) fn discover_all(definitions: &[Definition]) -> (Discovery, Vec<VersioningDiagnostic>) {
    let mut visitor = DiscoverVisitor {
        discovery: Discovery {
            version_enum: None,
            saw_lifecycle: Vec::new(),
            version_parameter: None,
        },
        errors: Vec::new(),
    };
    let _ = visitor.walk(definitions, "", Availability::full());
    (visitor.discovery, visitor.errors)
}

/// Collects every lifecycle violation of a versioned specification without aborting.
pub(crate) struct LifecycleCollector<'a> {
    pub(crate) timeline: &'a VersionTimeline,
    pub(crate) errors: Vec<VersioningDiagnostic>,
}

impl VersionVisitor for LifecycleCollector<'_> {
    fn enter(&mut self, site: &Site<'_>, parent: Availability) -> ParserResult<Availability> {
        Ok(availability_into(
            site,
            parent,
            self.timeline,
            &mut self.errors,
        ))
    }
}
