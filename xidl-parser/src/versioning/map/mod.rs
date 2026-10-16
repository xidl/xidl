//! Language-neutral facts describing adjacent API-version changes.

mod collect;
mod compare;
mod helpers;
mod model;

use super::{VersionId, VersioningRecord};
use crate::error::ParserResult;
use crate::hir::Specification;
use serde_json;

pub use model::{Change, ChangeKind, MapStep, MapVersion, TypeChange, VersionMap};

/// Computes the complete, unselected version map for a validated contract.
pub fn from_spec(
    spec: &Specification,
    record: &VersioningRecord,
    contract: impl Into<String>,
) -> ParserResult<VersionMap> {
    let versions = record
        .timeline()
        .iter()
        .map(|(_, version)| MapVersion {
            id: version.ident.clone(),
            value: version.value.clone(),
        })
        .collect::<Vec<_>>();
    let mut steps = Vec::new();
    for index in 0..versions.len().saturating_sub(1) {
        let from_id = VersionId::from_index(index);
        let to_id = VersionId::from_index(index + 1);
        let from = record.project(spec, from_id)?;
        let to = record.project(spec, to_id)?;
        steps.push(MapStep {
            from: versions[index].id.clone(),
            to: versions[index + 1].id.clone(),
            types: compare::compare_types(
                &collect::collect_types(&from),
                &collect::collect_types(&to),
            ),
            operations: compare::compare_operations(
                &collect::collect_operations(&from)?,
                &collect::collect_operations(&to)?,
            ),
        });
    }
    Ok(VersionMap {
        format: 1,
        contract: contract.into(),
        versions,
        steps,
    })
}

/// Serializes a version map with the repository's canonical JSON policy.
pub fn to_json(map: &VersionMap) -> Result<String, serde_json::Error> {
    let mut content = serde_json::to_string_pretty(map)?;
    content.push('\n');
    Ok(content)
}
