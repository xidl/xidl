//! Per-version projection of a versioned specification.

mod compound;
mod interface;
mod projector;
mod references;

use super::VersioningError;
use super::lifecycle::is_versioning_annotation;
use super::timeline::{VersionId, VersionTimeline};
use crate::error::ParserResult;
use crate::hir::{Annotation, Definition, ModuleDcl, Pragma, Specification};

use super::lifecycle::Availability;
use projector::Projector;

/// Projects one version of a versioned specification.
///
/// The result keeps top-level pragmas, replaces the document version with the
/// projected version value, and wraps the projected declarations in
/// `module <version-ident> { ... }`. Lifecycle annotations and the `@version`
/// enum are removed.
pub(crate) fn project_version(
    spec: &Specification,
    timeline: &VersionTimeline,
    version: VersionId,
) -> ParserResult<Specification> {
    let Some(def) = timeline.get(version) else {
        return Err(VersioningError::UnknownVersion {
            index: version.index(),
        }
        .into());
    };
    let mut projector = Projector::new(timeline, version, def);
    let mut pragmas = Vec::new();
    let mut body = Vec::new();
    for definition in &spec.0 {
        if let Definition::Pragma(_) = definition {
            pragmas.push(definition.clone());
            continue;
        }
        if let Some(projected) = projector.definition(definition, Availability::full(), "")? {
            body.push(projected);
        }
    }
    projector.check_references()?;
    projector.prune_raises(&mut body);
    let mut definitions = version_pragmas(pragmas, &def.value);
    definitions.push(Definition::ModuleDcl(ModuleDcl {
        annotations: Vec::new(),
        ident: def.ident.clone(),
        definition: body,
    }));
    let mut projected = Specification(definitions);
    crate::semantic::analyze(&mut projected)?;
    Ok(projected)
}

/// Merges several projected versions into one specification with one module per
/// version, in timeline order.
///
/// The document version pragma is dropped: a merged contract describes several
/// versions at once and no single version value applies to it.
pub(crate) fn merge_versions(
    spec: &Specification,
    timeline: &VersionTimeline,
    versions: &[VersionId],
) -> ParserResult<Specification> {
    let mut definitions = Vec::new();
    for (index, version) in versions.iter().enumerate() {
        let projected = project_version(spec, timeline, *version)?;
        for definition in projected.0 {
            match definition {
                Definition::Pragma(Pragma::XidlcOpenApiVersion(_)) => {}
                Definition::Pragma(pragma) => {
                    if index == 0 {
                        definitions.push(Definition::Pragma(pragma));
                    }
                }
                definition => definitions.push(definition),
            }
        }
    }
    Ok(Specification(definitions))
}

fn version_pragmas(pragmas: Vec<Definition>, value: &str) -> Vec<Definition> {
    let mut out = Vec::new();
    let mut replaced = false;
    for definition in pragmas {
        if let Definition::Pragma(Pragma::XidlcOpenApiVersion(_)) = &definition {
            if !replaced {
                out.push(Definition::Pragma(Pragma::XidlcOpenApiVersion(
                    value.to_string(),
                )));
                replaced = true;
            }
            continue;
        }
        out.push(definition);
    }
    if !replaced {
        out.push(Definition::Pragma(Pragma::XidlcOpenApiVersion(
            value.to_string(),
        )));
    }
    out
}
fn strip_annotations(annotations: &[Annotation]) -> Vec<Annotation> {
    annotations
        .iter()
        .filter(|annotation| !is_versioning_annotation(annotation))
        .cloned()
        .collect()
}
