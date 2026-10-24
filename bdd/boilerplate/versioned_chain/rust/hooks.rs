use crate::gen::version_adapters::*;
use crate::gen::{v2026_06_01, v2026_10_01, v2027_01_01};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};

fn unreadable_agenda(source: &v2026_06_01::Booking) -> UnsupportedProjection {
    UnsupportedProjection {
        version: Version::V20260601.id().into(),
        direction: Direction::Upcast,
        type_path: "Booking.agenda".into(),
        reason: format!("unreadable agenda for booking {}", source.id),
    }
}

/// Storage only moves forward: each booking row keeps what older releases
/// still read. The generated chain never touches this; it receives readers.
#[derive(Clone, Default)]
pub struct Storage {
    pub agenda_texts: BTreeMap<String, String>,
    pub rosters: BTreeMap<String, Vec<String>>,
}

impl StoredValues for Storage {
    fn booking_agenda(&self, source: &v2026_10_01::Booking) -> Option<String> {
        self.agenda_texts.get(&source.id).cloned()
    }
    fn booking_roster(&self, source: &v2027_01_01::Booking) -> Option<Vec<String>> {
        self.rosters.get(&source.id).cloned()
    }
}

#[derive(Default)]
pub struct Hooks {
    pub up_from_v1: AtomicUsize,
    pub down_to_v1: AtomicUsize,
    pub up_from_v2: AtomicUsize,
    pub down_to_v2: AtomicUsize,
}

impl VersionAdapters for Hooks {
    /// The oldest release packs the plan into one text block; the application
    /// states what the Head contract expects and acknowledges the retired text.
    fn booking_upcast_from_v_2026_06_01(
        &self,
        source: &v2026_06_01::Booking,
    ) -> VersionedResult<BookingUpcastFromV20260601Decision> {
        self.up_from_v1.fetch_add(1, Ordering::SeqCst);
        let (title, minutes) = source
            .agenda
            .split_once(';')
            .ok_or_else(|| unreadable_agenda(source))?;
        // The duration is a plain non-negative integer: signed, overflowing or
        // repeated-separator encodings are refused, matching every other
        // generated language.
        if minutes.is_empty() || !minutes.chars().all(|c| c.is_ascii_digit()) {
            return Err(unreadable_agenda(source));
        }
        let minutes = minutes.parse().map_err(|_| unreadable_agenda(source))?;
        Ok(BookingUpcastFromV20260601Decision {
            items: vec![v2026_10_01::AgendaEntry {
                title: title.into(),
                minutes,
            }],
            locked: false,
            agenda: IGNORED,
        })
    }

    fn booking_downcast_to_v_2026_06_01(
        &self,
        source: &v2026_10_01::Booking,
        stored: &dyn StoredValues,
    ) -> VersionedResult<BookingDowncastToV20260601Decision> {
        self.down_to_v1.fetch_add(1, Ordering::SeqCst);
        let Some(agenda) = stored.booking_agenda(source) else {
            return Err(UnsupportedProjection {
                version: Version::V20261001.id().into(),
                direction: Direction::Downcast,
                type_path: "Booking.agenda".into(),
                reason: format!("no stored agenda for booking {}", source.id),
            });
        };
        Ok(BookingDowncastToV20260601Decision { agenda })
    }

    fn booking_upcast_from_v_2026_10_01(
        &self,
        _source: &v2026_10_01::Booking,
    ) -> VersionedResult<BookingUpcastFromV20261001Decision> {
        self.up_from_v2.fetch_add(1, Ordering::SeqCst);
        Ok(BookingUpcastFromV20261001Decision { roster: IGNORED })
    }

    fn booking_downcast_to_v_2026_10_01(
        &self,
        source: &v2027_01_01::Booking,
        stored: &dyn StoredValues,
    ) -> VersionedResult<BookingDowncastToV20261001Decision> {
        self.down_to_v2.fetch_add(1, Ordering::SeqCst);
        let Some(roster) = stored.booking_roster(source) else {
            return Err(UnsupportedProjection {
                version: Version::V20270101.id().into(),
                direction: Direction::Downcast,
                type_path: "Booking.roster".into(),
                reason: format!("no stored roster for booking {}", source.id),
            });
        };
        Ok(BookingDowncastToV20261001Decision { roster })
    }
}
