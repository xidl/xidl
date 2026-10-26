use crate::gen::version_adapters::{
    downcast_profile, profile_downcast_to_v_2026_01_01, profile_downcast_to_v_2026_04_01,
    profile_upcast_from_v_2026_01_01, profile_upcast_from_v_2026_04_01, ProfileProjection,
    Version, NEEDS,
};
use crate::gen::{v2026_01_01, v2026_04_01, v2026_07_01};
use crate::hooks::{Hooks, Storage};
use std::collections::BTreeMap;
use std::sync::atomic::Ordering;

fn storage_for_oldest() -> Storage {
    Storage {
        created_at: BTreeMap::from([("p1".into(), 1_700_000_000)]),
        first_names: BTreeMap::from([("p1".into(), "Ann".into())]),
        last_names: BTreeMap::from([("p1".into(), "Lee".into())]),
        notes: BTreeMap::from([(
            "p1".into(),
            v2026_04_01::Note { text: "prefers email".into() },
        )]),
        statuses: BTreeMap::from([("p1".into(), "sent".into())]),
        recipients: BTreeMap::from([("p1".into(), "anna@example.test".into())]),
    }
}

fn oldest() -> v2026_01_01::Profile {
    v2026_01_01::Profile {
        created_at: 1_700_000_000,
        first_name: "Ann".into(),
        id: "p1".into(),
        last_name: "Lee".into(),
        note: Some(v2026_01_01::Note { text: "prefers email".into() }),
        recipient: "anna@example.test".into(),
        status: "sent".into(),
        title: "founder".into(),
    }
}

fn head() -> v2026_07_01::Profile {
    v2026_07_01::Profile {
        created_at_utc: "2026-06-01T12:00:00.000Z".into(),
        display_name: "Dr. Ann Lee".into(),
        fallback: Some(false),
        id: "p1".into(),
        notes: vec![
            v2026_07_01::Note { text: "prefers email".into() },
            v2026_07_01::Note { text: "vip".into() },
        ],
        read_at: Some("2026-06-02T08:00:00.000Z".into()),
        tier: "premium".into(),
        title: "founder".into(),
        to: v2026_07_01::Route {
            handle: "anna@example.test".into(),
            protocol: Some(v2026_07_01::RouteProtocol::chat),
        },
    }
}

fn storage_for_head() -> Storage {
    Storage {
        created_at: BTreeMap::from([("p1".into(), 1_780_315_200)]),
        first_names: BTreeMap::from([("p1".into(), "Ann".into())]),
        last_names: BTreeMap::from([("p1".into(), "Lee".into())]),
        notes: BTreeMap::from([(
            "p1".into(),
            v2026_04_01::Note { text: "prefers email".into() },
        )]),
        statuses: BTreeMap::from([("p1".into(), "read".into())]),
        recipients: BTreeMap::from([("p1".into(), "anna@example.test".into())]),
    }
}

/// Every transform shape upcasts, downcasts and round-trips without
/// inventing facts.
pub fn assert_transforms() {
    assert_eq!(NEEDS.len(), 1);
    assert_eq!(NEEDS[0].0, "Profile");
    let per_release = NEEDS[0].1;
    assert_eq!(per_release[0], ("2026-07-01", &[][..]));
    assert_eq!(
        per_release[1],
        ("2026-04-01", &["created_at", "first_name", "last_name", "note", "status"][..])
    );
    assert_eq!(
        per_release[2],
        ("2026-01-01", &["created_at", "first_name", "last_name", "note", "recipient", "status"][..])
    );

    let hooks = Hooks::default();

    // E8 inverted: the upcast states the old behaviour (true), never the new
    // default (false); the flat recipient becomes a route with no protocol.
    let to_v2 = profile_upcast_from_v_2026_01_01(&oldest(), &hooks).unwrap();
    assert_eq!(to_v2.fallback, Some(true), "the old behaviour is stated");
    assert_eq!(to_v2.to.handle, "anna@example.test");
    assert_eq!(to_v2.to.protocol, None);

    // The second step merges into Head: E6, E3 and E7 are mechanical there.
    let to_head = profile_upcast_from_v_2026_04_01(&to_v2, &hooks).unwrap();
    assert_eq!(to_head.created_at_utc, "2023-11-14T22:13:20.000Z");
    assert_eq!(to_head.display_name, "Ann Lee");
    assert_eq!(to_head.notes.len(), 1);
    assert_eq!(to_head.read_at, None);
    // E12: the decision states the tier; no default exists for it.
    assert_eq!(to_head.tier, "standard");

    // E2 + E8 downward: the route reads back as the stored recipient.
    let storage = storage_for_oldest();
    let down_v1 = profile_downcast_to_v_2026_01_01(&to_v2, &storage, &hooks).unwrap();
    assert_eq!(down_v1.recipient, "anna@example.test");
    // Round-trip rule one: read, upcast, downcast — the system is unchanged.
    let full = downcast_profile(Version::V20260101, to_head.clone(), &storage, &hooks).unwrap();
    let ProfileProjection::V20260101(full_value) = full else {
        panic!("the oldest projection lands in its own module");
    };
    assert_eq!(full_value.created_at, 1_700_000_000, "E6 floors to the stored seconds");
    assert_eq!(full_value.recipient, "anna@example.test");
    assert_eq!(full_value.status, "sent");
    assert_eq!(full_value.first_name, "Ann");
    assert_eq!(full_value.note.as_ref().unwrap().text, "prefers email");

    // E5 upward: a retired enum value can never invent its timestamp.
    let read = v2026_01_01::Profile { status: "read".into(), ..oldest() };
    let refusal = profile_upcast_from_v_2026_01_01(&read, &hooks).unwrap_err();
    assert_eq!(refusal.type_path, "Profile.read_at");

    // E4: the newest protocol value has no image in 2026-04, and the chain
    // refuses there for every older caller because the step sits on the path.
    let head_storage = storage_for_head();
    let chat_refusal = match downcast_profile(Version::V20260401, head(), &head_storage, &hooks) {
        Err(error) => error,
        Ok(_) => panic!("the newest protocol value must not project"),
    };
    assert_eq!(chat_refusal.type_path, "Route.protocol");
    assert_eq!(hooks.protocol_refusals.load(Ordering::SeqCst), 1);
    let chain_refusal = match downcast_profile(Version::V20260101, head(), &head_storage, &hooks) {
        Err(error) => error,
        Ok(_) => panic!("the chain must stop at the first refusing release"),
    };
    assert_eq!(chain_refusal.type_path, "Route.protocol");
    assert_eq!(hooks.protocol_refusals.load(Ordering::SeqCst), 2);

    // Without the newest value the whole chain walks unchanged, and E6 floors
    // by declaration while E5 renders the read time as the retired value.
    let mut seconds_only = head();
    seconds_only.to.protocol = Some(v2026_07_01::RouteProtocol::relay);
    let down_v2 =
        profile_downcast_to_v_2026_04_01(&seconds_only, &head_storage, &hooks).unwrap();
    assert_eq!(down_v2.created_at, 1_780_315_200);
    assert_eq!(down_v2.first_name, "Ann");
    assert_eq!(down_v2.last_name, "Lee");
    assert_eq!(down_v2.status, "read");
    assert_eq!(down_v2.note.as_ref().unwrap().text, "prefers email");

    // E3 downward: without the stored pieces the split refuses instead of
    // guessing, and the chain stops at the first missing fact.
    let anonymous = v2026_07_01::Profile { id: "p-anon".into(), ..head() };
    let refusal = profile_downcast_to_v_2026_04_01(&anonymous, &head_storage, &hooks).unwrap_err();
    assert_eq!(refusal.type_path, "Profile.created_at");
    let empty = Storage::default();
    let refusal = profile_downcast_to_v_2026_04_01(&anonymous, &empty, &hooks).unwrap_err();
    assert_eq!(refusal.type_path, "Profile.created_at");

    // E7 downward: nothing stored means nothing served, an explicit OMIT.
    let mut noteless_storage = storage_for_head();
    noteless_storage.notes.clear();
    let down_v2 =
        profile_downcast_to_v_2026_04_01(&seconds_only, &noteless_storage, &hooks).unwrap();
    assert_eq!(down_v2.note, None);

    assert_eq!(hooks.up_from_v1.load(Ordering::SeqCst), 2);
    assert_eq!(hooks.down_to_v1.load(Ordering::SeqCst), 2);
    assert_eq!(hooks.up_from_v2.load(Ordering::SeqCst), 1);
    // Three direct decisions plus four chain walks that pass through the
    // 2026-04 step (the full round trip, the two chat refusals, the noteless read).
    assert_eq!(hooks.down_to_v2.load(Ordering::SeqCst), 7);
}
