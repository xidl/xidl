use crate::gen::{v1, v2, v3, version_adapters::*};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
pub struct Hooks {
    pub calls: AtomicUsize,
    pub errors: AtomicUsize,
    pub reject_downcast: bool,
    pub reject_patch: bool,
}

/// Storage keeps what Head no longer carries while older versions still read it.
/// Readers resolve identity from the Head instance; the chain never touches storage.
#[derive(Clone, Default)]
pub struct Storage {
    pub handling_notes: BTreeMap<String, String>,
    pub traces: BTreeMap<String, String>,
}

impl StoredValues for Storage {
    fn optional_change_note(&self, source: &v2::OptionalChange) -> Option<String> {
        source.note.as_deref().filter(|note| *note == "kept").map(str::to_string)
    }
    /// Never stored: the hook derives the retired reference from the tracking number.
    fn parcel_legacy(&self, _: &v2::Parcel) -> Option<String> { None }
    fn parcel_handling_note(&self, source: &v3::Parcel) -> Option<String> {
        self.handling_notes.get(&source.id).cloned()
    }
    /// The trace header left the Head response; storage keeps it per traced id.
    fn depot_trace_response_trace_id(&self, return_value: &String) -> Option<String> {
        self.traces.get(return_value).cloned()
    }
}

pub fn refusal(version: &str, direction: Direction, path: &str) -> UnsupportedProjection {
    UnsupportedProjection {
        version: version.into(), direction, type_path: path.into(),
        reason: "cannot represent this business value".into(),
    }
}

impl VersionAdapters for Hooks {
    fn choice_downcast_to_v_1(&self, source: &ChoiceDowncastToV1Changes<'_>) -> VersionedResult<v1::Choice> {
        let ChoiceDowncastToV1Changes::count(value) = source;
        Ok(v1::Choice::new_text(value.to_string()))
    }
    fn optional_change_upcast_from_v_1(&self, source: &v1::OptionalChange) -> VersionedResult<OptionalChangeUpcastFromV1Decision> {
        Ok(OptionalChangeUpcastFromV1Decision { note: source.note.clone(), ignored_note: IGNORED })
    }
    fn optional_change_downcast_to_v_1(&self, source: &v2::OptionalChange, stored: &dyn StoredValues) -> VersionedResult<OptionalChangeDowncastToV1Decision> {
        Ok(OptionalChangeDowncastToV1Decision { note: stored.optional_change_note(source) })
    }
    fn parcel_upcast_from_v_1(&self, source: &v1::Parcel) -> VersionedResult<ParcelUpcastFromV1Decision> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(ParcelUpcastFromV1Decision {
            tracking: source.legacy.clone(), carrier: "postal".into(), note: None,
            courier_note: None, legacy: IGNORED,
        })
    }
    fn parcel_downcast_to_v_1(&self, source: &v2::Parcel, _: &dyn StoredValues) -> VersionedResult<ParcelDowncastToV1Decision> {
        if self.reject_downcast { return Err(refusal("2026-01", Direction::Downcast, "Parcel")); }
        Ok(ParcelDowncastToV1Decision { legacy: source.tracking.clone() })
    }
    fn parcel_downcast_to_v_2(&self, source: &v3::Parcel, stored: &dyn StoredValues) -> VersionedResult<ParcelDowncastToV2Decision> {
        Ok(ParcelDowncastToV2Decision { handling_note: stored.parcel_handling_note(source) })
    }
    fn depot_trace_response_downcast_to_v_2(&self, source: &String, stored: &dyn StoredValues) -> VersionedResult<DepotTraceResponseDowncastToV2Decision> {
        // v1 and v2 declare the header required: a missing row refuses instead of inventing one.
        let trace_id = stored.depot_trace_response_trace_id(source).ok_or_else(|| refusal("2026-04", Direction::Downcast, "Depot.trace"))?;
        Ok(DepotTraceResponseDowncastToV2Decision { trace_id })
    }
    fn parcel_upcast_from_v_2(&self, source: &v2::Parcel) -> VersionedResult<ParcelUpcastFromV2Decision> {
        if source.tracking == "reject-v2" { return Err(refusal("2026-04", Direction::Upcast, "Parcel")); }
        Ok(ParcelUpcastFromV2Decision { inspected: None, audit: "checked".into(), handling_note: IGNORED })
    }
    fn parcel_patch_upcast_from_v_2(&self, source: &v2::ParcelPatch, _current: &v3::Parcel) -> VersionedResult<ParcelPatchUpcastFromV2Decision> {
        if self.reject_patch { return Err(refusal("2026-04", Direction::Upcast, "ParcelPatch")); }
        Ok(ParcelPatchUpcastFromV2Decision {
            handling_note: IGNORED,
            stored: DepotUpdateStored { handling_note: source.handling_note.clone() },
        })
    }
    fn state_downcast_to_v_1(&self, _: &StateDowncastToV1Changes) -> VersionedResult<v1::State> {
        Err(refusal("2026-01", Direction::Downcast, "State"))
    }
    fn state_downcast_to_v_2(&self, _: &StateDowncastToV2Changes) -> VersionedResult<v2::State> {
        Ok(v2::State::shipping)
    }
    fn depot_get_busy_error_downcast_to_v_1(&self, source: &v2::Busy) -> VersionedResult<v1::DepotGetError> {
        self.errors.fetch_add(1, Ordering::SeqCst);
        Ok(v1::DepotGetError::NotFound(v1::NotFound { id: source.reason.clone() }))
    }
    fn depot_get_framework_error_downcast_to_v_2(&self, source: &v3::Framework) -> VersionedResult<v2::DepotGetError> {
        self.errors.fetch_add(1, Ordering::SeqCst);
        if source.reason == "refuse" { return Err(refusal("2026-04", Direction::Downcast, "Framework")); }
        Ok(v2::DepotGetError::Busy(v2::Busy { reason: source.reason.clone() }))
    }
    fn depot_get_denied_error_downcast_to_v_2(&self, source: &v3::Denied) -> VersionedResult<v2::DepotGetError> {
        self.errors.fetch_add(1, Ordering::SeqCst);
        Ok(v2::DepotGetError::Busy(v2::Busy { reason: source.reason.clone() }))
    }
    fn depot_update_busy_error_downcast_to_v_1(&self, source: &v2::Busy) -> VersionedResult<v1::DepotUpdateError> {
        self.errors.fetch_add(1, Ordering::SeqCst);
        Ok(v1::DepotUpdateError::NotFound(v1::NotFound { id: source.reason.clone() }))
    }
    fn depot_update_framework_error_downcast_to_v_2(&self, source: &v3::Framework) -> VersionedResult<v2::DepotUpdateError> {
        self.errors.fetch_add(1, Ordering::SeqCst);
        Ok(v2::DepotUpdateError::Busy(v2::Busy { reason: source.reason.clone() }))
    }
    fn depot_update_denied_error_downcast_to_v_2(&self, source: &v3::Denied) -> VersionedResult<v2::DepotUpdateError> {
        self.errors.fetch_add(1, Ordering::SeqCst);
        Ok(v2::DepotUpdateError::Busy(v2::Busy { reason: source.reason.clone() }))
    }
}
