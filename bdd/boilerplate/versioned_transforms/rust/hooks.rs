use crate::gen::version_adapters::*;
use crate::gen::{v2026_01_01, v2026_04_01, v2026_07_01};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};

/// What older releases still read, keyed by profile id.
#[derive(Clone, Default)]
pub struct Storage {
    pub created_at: BTreeMap<String, i32>,
    pub first_names: BTreeMap<String, String>,
    pub last_names: BTreeMap<String, String>,
    pub notes: BTreeMap<String, v2026_04_01::Note>,
    pub statuses: BTreeMap<String, String>,
    pub recipients: BTreeMap<String, String>,
}

impl StoredValues for Storage {
    fn profile_recipient(&self, source: &v2026_04_01::Profile) -> Option<String> {
        self.recipients.get(&source.id).cloned()
    }
    fn profile_created_at(&self, source: &v2026_07_01::Profile) -> Option<i32> {
        self.created_at.get(&source.id).cloned()
    }
    fn profile_first_name(&self, source: &v2026_07_01::Profile) -> Option<String> {
        self.first_names.get(&source.id).cloned()
    }
    fn profile_last_name(&self, source: &v2026_07_01::Profile) -> Option<String> {
        self.last_names.get(&source.id).cloned()
    }
    fn profile_note(&self, source: &v2026_07_01::Profile) -> Option<v2026_04_01::Note> {
        self.notes.get(&source.id).cloned()
    }
    fn profile_status(&self, source: &v2026_07_01::Profile) -> Option<String> {
        self.statuses.get(&source.id).cloned()
    }
}

#[derive(Default)]
pub struct Hooks {
    pub up_from_v1: AtomicUsize,
    pub down_to_v1: AtomicUsize,
    pub up_from_v2: AtomicUsize,
    pub down_to_v2: AtomicUsize,
    pub protocol_refusals: AtomicUsize,
}

fn unreadable_read(id: &str, version: &str) -> UnsupportedProjection {
    UnsupportedProjection {
        version: version.into(),
        direction: Direction::Upcast,
        type_path: "Profile.read_at".into(),
        reason: format!("a read time for profile {id} can never be invented"),
    }
}

impl VersionAdapters for Hooks {
    /// E8 inverted: older callers' behaviour matched true while the new member
    /// defaults to false, so the upcast states true and never the default.
    fn profile_upcast_from_v_2026_01_01(
        &self,
        source: &v2026_01_01::Profile,
    ) -> VersionedResult<ProfileUpcastFromV20260101Decision> {
        self.up_from_v1.fetch_add(1, Ordering::SeqCst);
        // E5 upward: "read" retired into a timestamp nobody can invent.
        if source.status == "read" {
            return Err(unreadable_read(&source.id, "2026-01-01"));
        }
        Ok(ProfileUpcastFromV20260101Decision {
            fallback: true,
            to: v2026_04_01::Route {
                handle: source.recipient.clone(),
                protocol: None,
            },
            recipient: IGNORED,
        })
    }

    fn profile_downcast_to_v_2026_01_01(
        &self,
        source: &v2026_04_01::Profile,
        stored: &dyn StoredValues,
    ) -> VersionedResult<ProfileDowncastToV20260101Decision> {
        self.down_to_v1.fetch_add(1, Ordering::SeqCst);
        let Some(recipient) = stored.profile_recipient(source) else {
            return Err(UnsupportedProjection {
                version: "2026-01-01".into(),
                direction: Direction::Downcast,
                type_path: "Profile.recipient".into(),
                reason: format!("no stored recipient for profile {}", source.id),
            });
        };
        Ok(ProfileDowncastToV20260101Decision { recipient })
    }

    fn profile_upcast_from_v_2026_04_01(
        &self,
        source: &v2026_04_01::Profile,
    ) -> VersionedResult<ProfileUpcastFromV20260401Decision> {
        self.up_from_v2.fetch_add(1, Ordering::SeqCst);
        if source.status == "read" {
            return Err(unreadable_read(&source.id, "2026-04-01"));
        }
        // E6 upward: seconds to RFC 3339, deterministic and lossless. E3: the
        // merge is mechanical. E7: one optional note becomes a one-entry list.
        Ok(ProfileUpcastFromV20260401Decision {
            created_at_utc: epoch_to_rfc3339(source.created_at),
            display_name: format!("{} {}", source.first_name, source.last_name),
            notes: source
                .note
                .iter()
                .map(|note| v2026_07_01::Note {
                    text: note.text.clone(),
                })
                .collect(),
            read_at: None,
            // E12: a required member arrives at Head; the decision states the
            // application's rule for callers who cannot name the member yet.
            tier: "standard".into(),
            created_at: IGNORED,
            first_name: IGNORED,
            last_name: IGNORED,
            note: IGNORED,
            status: IGNORED,
        })
    }

    fn profile_downcast_to_v_2026_04_01(
        &self,
        source: &v2026_07_01::Profile,
        stored: &dyn StoredValues,
    ) -> VersionedResult<ProfileDowncastToV20260401Decision> {
        self.down_to_v2.fetch_add(1, Ordering::SeqCst);
        let refuse = |type_path: &str, reason: String| UnsupportedProjection {
            version: "2026-04-01".into(),
            direction: Direction::Downcast,
            type_path: type_path.into(),
            reason,
        };
        let Some(created_at) = stored.profile_created_at(source) else {
            return Err(refuse(
                "Profile.created_at",
                format!("no stored creation time for profile {}", source.id),
            ));
        };
        let Some(first_name) = stored.profile_first_name(source) else {
            return Err(refuse(
                "Profile.first_name",
                format!("the display name of profile {} cannot be split", source.id),
            ));
        };
        let Some(last_name) = stored.profile_last_name(source) else {
            return Err(refuse(
                "Profile.last_name",
                format!("the display name of profile {} cannot be split", source.id),
            ));
        };
        let Some(status) = stored.profile_status(source) else {
            return Err(refuse(
                "Profile.status",
                format!("no stored status for profile {}", source.id),
            ));
        };
        Ok(ProfileDowncastToV20260401Decision {
            created_at,
            first_name,
            last_name,
            // E7 downward: nothing stored means nothing served, an explicit
            // OMIT instead of a fabricated note.
            note: stored.profile_note(source),
            status,
        })
    }

    /// E10: the oldest contracts cannot name the refinement, so the downgrade
    /// keeps the committed status on the runtime channel instead of
    /// translating a sealed profile into a missing one.
    fn directory_profile_sealed_error_downcast_to_v_2026_04_01(
        &self,
        source: &v2026_07_01::Sealed,
    ) -> VersionedResult<v2026_04_01::DirectoryProfileError> {
        Ok(v2026_04_01::DirectoryProfileError::Framework(
            xidl_rust_axum::Error::new(409, format!("profile {} is sealed", source.id)),
        ))
    }

    /// E4: the newest value simply has no image one release earlier.
    fn route_protocol_downcast_to_v_2026_04_01(
        &self,
        source: &RouteProtocolDowncastToV20260401Changes,
    ) -> VersionedResult<v2026_04_01::RouteProtocol> {
        self.protocol_refusals.fetch_add(1, Ordering::SeqCst);
        Err(UnsupportedProjection {
            version: "2026-04-01".into(),
            direction: Direction::Downcast,
            type_path: "Route.protocol".into(),
            reason: "chat has no image in the 2026-04-01 contract".into(),
        })
    }
}

pub fn epoch_to_rfc3339(seconds: i32) -> String {
    // A minimal deterministic conversion for the corpus timestamps: every
    // value used here is a whole second after 1970-01-01T00:00:00Z.
    let days = seconds.div_euclid(86_400);
    let rest = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.000Z",
        rest / 3_600,
        (rest % 3_600) / 60,
        rest % 60
    )
}

fn civil_from_days(days: i32) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Floors an RFC 3339 timestamp to whole seconds; the declared lossy direction
/// of the precision change.
pub fn rfc3339_to_epoch(value: &str) -> Option<i32> {
    let parsed = chrono_millis(value)?;
    Some((parsed / 1000) as i32)
}

fn chrono_millis(value: &str) -> Option<i64> {
    let (date, rest) = value.split_once('T')?;
    let rest = rest.strip_suffix('Z')?;
    let (day, time) = (parse_date(date)?, parse_time(rest)?);
    Some(day * 86_400 + time)
}

fn parse_date(date: &str) -> Option<i64> {
    let mut parts = date.split('-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: i64 = parts.next()?.parse().ok()?;
    let day: i64 = parts.next()?.parse().ok()?;
    let leap = |y: i64| y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let cumulative = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let mut days = 365 * (year - 1970) + (1970..year).filter(|y| leap(*y)).count() as i64;
    days += cumulative[(month - 1).clamp(0, 11) as usize];
    if month > 2 && leap(year) {
        days += 1;
    }
    Some(days + day - 1)
}

fn parse_time(time: &str) -> Option<i64> {
    let mut parts = time.split(':');
    let hour: i64 = parts.next()?.parse().ok()?;
    let minute: i64 = parts.next()?.parse().ok()?;
    let second: f64 = parts.next()?.parse().ok()?;
    Some(hour * 3_600 + minute * 60 + second as i64)
}
