use crate::gen::{v1, v2, v3, version_adapters::*};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
pub struct Hooks {
    pub calls: AtomicUsize,
    pub errors: AtomicUsize,
    pub reject_downcast: bool,
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
    fn optional_change_downcast_to_v_1(&self, source: &v2::OptionalChange) -> VersionedResult<OptionalChangeDowncastToV1Decision> {
        Ok(OptionalChangeDowncastToV1Decision { note: source.note.clone() })
    }
    fn parcel_upcast_from_v_1(&self, source: &v1::Parcel) -> VersionedResult<ParcelUpcastFromV1Decision> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(ParcelUpcastFromV1Decision {
            tracking: source.legacy.clone(), carrier: "postal".into(), note: None, legacy: IGNORED,
        })
    }
    fn parcel_downcast_to_v_1(&self, source: &v2::Parcel) -> VersionedResult<ParcelDowncastToV1Decision> {
        if self.reject_downcast { return Err(refusal("2026-01", Direction::Downcast, "Parcel")); }
        Ok(ParcelDowncastToV1Decision { legacy: source.tracking.clone() })
    }
    fn parcel_upcast_from_v_2(&self, source: &v2::Parcel) -> VersionedResult<ParcelUpcastFromV2Decision> {
        if source.tracking == "reject-v2" { return Err(refusal("2026-04", Direction::Upcast, "Parcel")); }
        Ok(ParcelUpcastFromV2Decision { audit: "checked".into() })
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
}
