use crate::gen::version_adapters::{
    booking_upcast_from_v_2026_06_01, booking_upcast_from_v_2026_10_01, downcast_booking,
    BookingProjection, Version, NEEDS,
};
use crate::gen::{v2026_06_01, v2027_01_01};
use crate::hooks::{Hooks, Storage};
use std::collections::BTreeMap;
use std::sync::atomic::Ordering;

/// The retirement report: which release still reads which wire name.
pub fn assert_needs_table() {
    assert_eq!(NEEDS.len(), 1);
    assert_eq!(NEEDS[0].0, "Booking");
    let per_release = NEEDS[0].1;
    assert_eq!(per_release.len(), 3);
    assert_eq!(per_release[0], ("2027-01-01", &[][..]));
    assert_eq!(per_release[1], ("2026-10-01", &["roster"][..]));
    assert_eq!(per_release[2], ("2026-06-01", &["agenda", "roster"][..]));
}

pub fn head() -> v2027_01_01::Booking {
    v2027_01_01::Booking {
        id: "b1".into(),
        title: "Kickoff".into(),
        items: vec![v2027_01_01::AgendaEntry { title: "Kickoff".into(), minutes: 60 }],
        organizer: Some("user-9".into()),
        locked: Some(true),
    }
}

pub fn storage() -> Storage {
    Storage {
        agenda_texts: BTreeMap::from([("b1".into(), "Kickoff;60".into())]),
        rosters: BTreeMap::from([("b1".into(), vec!["user-1".into(), "user-2".into()])]),
    }
}

pub fn assert_conversions(hooks: &Hooks, storage: &Storage) {
    // Head -> 2026-10-01: the roster returns from storage; organizer keeps the
    // role under its older source name with no decision anywhere.
    let to_v2 = downcast_booking(Version::V20261001, head(), storage, hooks).unwrap();
    let BookingProjection::V20261001(value) = to_v2 else {
        panic!("v2026_10_01 projects to its own module");
    };
    assert_eq!(value.roster, vec!["user-1".to_string(), "user-2".to_string()]);
    assert_eq!(value.owner.as_deref(), Some("user-9"));
    assert_eq!(value.locked, Some(true));
    assert_eq!(hooks.down_to_v2.load(Ordering::SeqCst), 1, "only the roster decision ran");
    assert_eq!(hooks.up_from_v1.load(Ordering::SeqCst) + hooks.up_from_v2.load(Ordering::SeqCst), 0);

    // Head -> 2026-06-01 walks both steps in one call.
    let to_v1 = downcast_booking(Version::V20260601, head(), storage, hooks).unwrap();
    let BookingProjection::V20260601(value) = to_v1 else {
        panic!("v2026_06_01 projects to its own module");
    };
    assert_eq!(value.agenda, "Kickoff;60", "the oldest release reads the stored agenda");
    assert_eq!(value.roster, vec!["user-1".to_string(), "user-2".to_string()]);
    assert_eq!(value.owner.as_deref(), Some("user-9"));
    assert_eq!(hooks.down_to_v1.load(Ordering::SeqCst), 1);

    // A row that storage never kept cannot be served: information only gets
    // dropped, never invented. The chain stops at the first missing fact.
    let orphan = v2027_01_01::Booking { id: "b-missing".into(), ..head() };
    let missing = match downcast_booking(Version::V20261001, orphan.clone(), storage, hooks) {
        Err(error) => error,
        Ok(_) => panic!("a booking without a stored roster must not project"),
    };
    assert_eq!(missing.type_path, "Booking.roster");
    let partial = Storage {
        agenda_texts: BTreeMap::new(),
        rosters: BTreeMap::from([("b-missing".into(), Vec::new())]),
    };
    let missing = match downcast_booking(Version::V20260601, orphan, &partial, hooks) {
        Err(error) => error,
        Ok(_) => panic!("a booking without a stored agenda must not project"),
    };
    assert_eq!(missing.type_path, "Booking.agenda");

    // The oldest caller upgrades: the implied locked value is stated, the
    // retired agenda is acknowledged, and owner arrives at Head as organizer.
    let v1 = v2026_06_01::Booking {
        id: "b1".into(),
        title: "Kickoff".into(),
        agenda: "Kickoff;60".into(),
        roster: vec!["user-1".into(), "user-2".into()],
        owner: Some("user-9".into()),
    };
    let to_v2 = booking_upcast_from_v_2026_06_01(&v1, hooks).unwrap();
    assert_eq!(hooks.up_from_v1.load(Ordering::SeqCst), 1);
    assert_eq!(to_v2.items.len(), 1);
    assert_eq!(to_v2.items[0].title, "Kickoff");
    assert_eq!(to_v2.items[0].minutes, 60);
    assert_eq!(to_v2.locked, Some(false), "the hook states the implied value");
    assert_eq!(to_v2.owner.as_deref(), Some("user-9"));
    assert_eq!(to_v2.roster, vec!["user-1".to_string(), "user-2".to_string()]);
    let to_head = booking_upcast_from_v_2026_10_01(&to_v2, hooks).unwrap();
    assert_eq!(to_head.organizer.as_deref(), Some("user-9"));
    assert_eq!(to_head.locked, Some(false));

    // An absent optional predecessor stays absent: the rename copies what is
    // there and invents nothing when the caller omitted it.
    let ownerless = v2026_06_01::Booking { owner: None, ..v1.clone() };
    let ownerless_v2 = booking_upcast_from_v_2026_06_01(&ownerless, hooks).unwrap();
    assert_eq!(
        ownerless_v2.owner, None,
        "the rename carries absence, never a fabricated owner"
    );
    let ownerless_head = booking_upcast_from_v_2026_10_01(&ownerless_v2, hooks).unwrap();
    assert_eq!(
        ownerless_head.organizer, None,
        "Head sees no organizer when no owner was ever named"
    );

    // An ambiguous text block is refused, not guessed: the sample encoding
    // cannot represent a title that contains the separator.
    let ambiguous = v2026_06_01::Booking { id: "b-amb".into(), agenda: "A;B;45".into(), ..v1.clone() };
    let refusal = match booking_upcast_from_v_2026_06_01(&ambiguous, hooks) {
        Err(error) => error,
        Ok(_) => panic!("an undecodable agenda must not upcast"),
    };
    assert_eq!(refusal.type_path, "Booking.agenda");

    // Write-then-read is a fixed point: the oldest wire shape survives the
    // trip to Head and back because storage kept the retired facts.
    let round_trip = downcast_booking(Version::V20260601, to_head, storage, hooks).unwrap();
    let BookingProjection::V20260601(value) = round_trip else {
        panic!("the fixed point projects to the oldest module");
    };
    assert_eq!(value, v1, "the oldest wire shape survives the round trip");
}
