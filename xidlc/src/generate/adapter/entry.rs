//! The opt-in entry point of the version-adapter stage.
//!
//! Versioned contracts keep rendering plain per-version output unless the
//! build turns adapters on; the timeline and the unprojected intervals
//! arrive through the generation properties. Language renderers consume
//! the plan this module derives.

use std::collections::HashMap;

use xidl_parser::rest_hir::RestHirDocument;
use xidl_parser::versioning::{self, MemberInterval, VersionDef};

use super::{AdapterPlan, derive, surface};
use crate::error::{IdlcError, IdlcResult};
use xidl_parser::hir;

/// Generation property that turns adapter generation on; versioned
/// contracts keep rendering plain per-version output unless it is set.
pub(crate) const ENABLE_VERSION_ADAPTERS_PROPERTY: &str = "enable_version_adapters";

/// Derives the version-adapter plan for an opted-in versioned contract.
///
/// Returns `None` when the contract is unversioned or the property is off,
/// so existing output stays byte-identical.
pub(crate) fn adapter_plan(
    spec: &hir::Specification,
    rest_hir: &RestHirDocument,
    props: &HashMap<String, serde_json::Value>,
) -> IdlcResult<Option<AdapterPlan>> {
    let enabled = props
        .get(ENABLE_VERSION_ADAPTERS_PROPERTY)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let timeline = props.get(versioning::API_VERSION_TIMELINE_PROPERTY);
    if !enabled || timeline.is_none() {
        return Ok(None);
    }
    let versions: Vec<VersionDef> = serde_json::from_value(
        timeline.cloned().unwrap_or(serde_json::Value::Null),
    )
    .map_err(|error| {
        IdlcError::fmt(format!(
            "version adapters: the declared timeline is malformed: {error}"
        ))
    })?;
    let intervals: Vec<MemberInterval> = serde_json::from_value(
        props
            .get(versioning::API_VERSION_INTERVALS_PROPERTY)
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    )
    .map_err(|error| {
        IdlcError::fmt(format!(
            "version adapters: the declaration intervals are malformed: {error}"
        ))
    })?;
    let surfaces = surface::collect(spec, rest_hir);
    derive(&surfaces, &versions, &intervals).map(Some)
}
