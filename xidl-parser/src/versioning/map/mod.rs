//! Language-neutral facts describing adjacent API-version changes.

mod collect;
mod compare;
mod declarations;
mod helpers;
mod model;

use super::{VersionId, VersioningRecord};
use crate::error::ParserResult;
use crate::hir::Specification;
use crate::semantic::IntegerImports;
use serde_json;
use std::collections::BTreeMap;

pub use model::{Change, ChangeKind, MapStep, MapVersion, TypeChange, VersionMap};

/// Source context used while comparing fully projected version contracts.
pub struct MapOptions {
    /// Contract name recorded in the artifact.
    pub contract: String,
    /// Whether the contract exposes HTTP operations.
    pub include_http_operations: bool,
    /// Immutable dependency declarations used by array dimensions.
    pub integer_imports: IntegerImports,
}

/// Computes the complete, unselected version map for a validated contract.
pub fn from_spec(
    spec: &Specification,
    record: &VersioningRecord,
    contract: impl Into<String>,
) -> ParserResult<VersionMap> {
    from_spec_with_http(
        spec,
        record,
        MapOptions {
            contract: contract.into(),
            include_http_operations: true,
            integer_imports: IntegerImports::default(),
        },
    )
}

/// Computes a version map while optionally collecting REST operation facts.
///
/// JSON-RPC generators pass `false`: a REST projection would otherwise invent
/// HTTP routes for ordinary RPC interfaces. Type and declaration facts remain
/// protocol-independent in both modes.
pub fn from_spec_with_http(
    spec: &Specification,
    record: &VersioningRecord,
    options: MapOptions,
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
    let declarations = declarations::SourceDeclarations::collect(spec, record.timeline())?;
    let projections = record
        .timeline()
        .iter()
        .map(|(id, _)| {
            let mut projected = record.project_source(spec, id)?;
            crate::semantic::analyze(&mut projected, &options.integer_imports)?;
            Ok(projected)
        })
        .collect::<ParserResult<Vec<_>>>()?;
    for index in 0..versions.len().saturating_sub(1) {
        let from = &projections[index];
        let to = &projections[index + 1];
        let mut from_types = collect::collect_types(from);
        let mut to_types = collect::collect_types(to);
        let from_id = VersionId::from_index(index);
        let to_id = VersionId::from_index(index + 1);
        declarations.stamp_types(&mut from_types, from_id)?;
        declarations.stamp_types(&mut to_types, to_id)?;
        let operations = if options.include_http_operations {
            let mut from_operations = collect::collect_operations(from)?;
            let mut to_operations = collect::collect_operations(to)?;
            declarations.stamp_operations(&mut from_operations, from_id)?;
            declarations.stamp_operations(&mut to_operations, to_id)?;
            compare::compare_operations(&from_operations, &to_operations)
        } else {
            BTreeMap::new()
        };
        steps.push(MapStep {
            from: versions[index].id.clone(),
            to: versions[index + 1].id.clone(),
            types: compare::compare_types(&from_types, &to_types),
            operations,
        });
    }
    Ok(VersionMap {
        format: 2,
        contract: options.contract,
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
