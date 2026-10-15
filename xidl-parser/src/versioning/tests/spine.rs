use super::{TIMELINE, spine_err, spine_of};
use crate::versioning::{CellKind, Evidence};
#[test]
fn adjacent_rename_is_a_confirmed_fact() {
    let text = format!(
        "{TIMELINE}
struct Pet {{
    @added(ApiVersions::v2025_01)
    @removed(ApiVersions::v2025_06)
    long age;

    @added(ApiVersions::v2025_06)
    @rename(\"age\")
    long age2_;
}};
"
    );
    let spine = spine_of(&text).expect("versioned");
    let cell = spine
        .correspondence
        .iter()
        .find(|cell| cell.container == "Pet" && cell.wire_name == "age")
        .expect("rename cell");
    assert_eq!(cell.kind, CellKind::Renamed);
    assert_eq!(cell.evidence, Evidence::ExplicitRename);
    assert_eq!(cell.from_subject.as_deref(), Some("Pet::age"));
    assert_eq!(cell.to_subject.as_deref(), Some("Pet::age2_"));
}

#[test]
fn gap_resume_with_rename_is_confirmed() {
    let text = format!(
        "{TIMELINE}
struct Pet {{
    @added(ApiVersions::v2025_01)
    @removed(ApiVersions::v2025_06)
    long age;

    @added(ApiVersions::v2026_01)
    @rename(\"age\")
    long age2_;
}};
"
    );
    let spine = spine_of(&text).expect("versioned");
    let cell = spine
        .correspondence
        .iter()
        .find(|cell| cell.from == "v2025_06" && cell.to == "v2026_01")
        .expect("gap cell");
    assert_eq!(cell.kind, CellKind::Added);
    assert_eq!(
        cell.evidence,
        Evidence::ResumesAfterGap {
            gap: 1,
            confirmed: true
        }
    );
}

#[test]
fn wire_coincidence_never_becomes_a_rename_fact() {
    // `user_id` and `userId` are different raw names whose camelCase wire
    // names coincide; without an explicit @rename the resume stays on the
    // review side.
    let text = "
@version
enum ApiVersions {
    v1,
    v2,
    v3,
};

@rename_all(\"camelCase\")
struct Pet {
    @added(ApiVersions::v1)
    @removed(ApiVersions::v2)
    long user_id;

    @added(ApiVersions::v3)
    long userId;
};
";
    let spine = spine_of(text).expect("versioned");
    let cell = spine
        .correspondence
        .iter()
        .find(|cell| cell.from == "v2" && cell.to == "v3")
        .expect("coincidence cell");
    assert_eq!(cell.kind, CellKind::Added);
    assert_eq!(
        cell.evidence,
        Evidence::ResumesAfterGap {
            gap: 1,
            confirmed: false
        }
    );
}

#[test]
fn type_and_optionality_changes_are_review_cells() {
    let text = format!(
        "{TIMELINE}
struct Pet {{
    @added(ApiVersions::v2025_01)
    @removed(ApiVersions::v2025_06)
    long age;

    @added(ApiVersions::v2025_06)
    @rename(\"age\")
    string age2_;

    @added(ApiVersions::v2025_01)
    @removed(ApiVersions::v2025_06)
    long weight;

    @added(ApiVersions::v2025_06)
    @optional
    @rename(\"weight\")
    long weight2_;
}};
"
    );
    let spine = spine_of(&text).expect("versioned");
    let type_cell = spine
        .correspondence
        .iter()
        .find(|cell| cell.wire_name == "age")
        .expect("type cell");
    assert_eq!(type_cell.kind, CellKind::TypeChanged);
    assert_eq!(type_cell.evidence, Evidence::Same, "no rename is claimed");
    let optional_cell = spine
        .correspondence
        .iter()
        .find(|cell| cell.wire_name == "weight")
        .expect("optional cell");
    assert_eq!(optional_cell.kind, CellKind::Optionalized);
    assert_eq!(optional_cell.evidence, Evidence::Same);
}

#[test]
fn overlapping_wire_names_are_rejected() {
    let text = format!(
        "{TIMELINE}
struct Pet {{
    long a;
    @rename(\"a\")
    long b;
}};
"
    );
    let error = spine_err(&text);
    assert!(
        error.contains("serialize as \"a\""),
        "unexpected error: {error}"
    );
}
#[test]
fn removed_facts_end_their_interval() {
    let text = format!(
        "{TIMELINE}
struct Pet {{
    @added(ApiVersions::v2025_01)
    @removed(ApiVersions::v2025_06)
    long age;
}};
"
    );
    let spine = spine_of(&text).expect("versioned");
    let cell = spine
        .correspondence
        .iter()
        .find(|cell| cell.from == "v2025_01" && cell.to == "v2025_06")
        .expect("the departure is a cell of the pair that loses it");
    assert_eq!(cell.kind, CellKind::Removed);
    assert_eq!(cell.evidence, Evidence::IntervalEnd);
    assert_eq!(cell.from_subject.as_deref(), Some("Pet::age"));
    assert_eq!(cell.to_subject, None);
    assert!(
        !spine
            .correspondence
            .iter()
            .any(|cell| cell.from == "v2025_06" && cell.to == "v2026_01"),
        "a fact gone from both versions carries no further cells"
    );
}

#[test]
fn unchanged_facts_stay_mechanical() {
    let text = format!(
        "{TIMELINE}
struct Pet {{
    long id;
}};
"
    );
    let spine = spine_of(&text).expect("versioned");
    let cell = spine
        .correspondence
        .iter()
        .find(|cell| cell.from == "v2025_01" && cell.to == "v2025_06")
        .expect("a shared field must join");
    assert_eq!(cell.kind, CellKind::Mechanical);
    assert_eq!(cell.evidence, Evidence::Same);
    assert_eq!(cell.from_subject.as_deref(), Some("Pet::id"));
    assert_eq!(cell.to_subject.as_deref(), Some("Pet::id"));
    assert_eq!(cell.wire_name, "id");
}

#[test]
fn added_facts_start_their_interval() {
    let text = format!(
        "{TIMELINE}
struct Pet {{
    long id;

    @added(ApiVersions::v2025_06)
    @optional string note;
}};
"
    );
    let spine = spine_of(&text).expect("versioned");
    let cell = spine
        .correspondence
        .iter()
        .find(|cell| cell.wire_name == "note")
        .expect("the arrival is a cell of the pair that gains it");
    assert_eq!(cell.from, "v2025_01");
    assert_eq!(cell.to, "v2025_06");
    assert_eq!(cell.kind, CellKind::Added);
    assert_eq!(cell.evidence, Evidence::IntervalStart);
    assert_eq!(cell.from_subject, None);
    assert_eq!(cell.to_subject.as_deref(), Some("Pet::note"));
}

#[test]
fn adjacent_wire_coincidence_is_name_shifted() {
    // `user_id` and `userId` are different declarations whose camelCase wire
    // names coincide on the adjacent pair; without @rename the continuation
    // stays a review-side NameShifted, never a rename fact.
    let text = "
@version
enum ApiVersions {
    v1,
    v2,
};

@rename_all(\"camelCase\")
struct Pet {
    @added(ApiVersions::v1)
    @removed(ApiVersions::v2)
    long user_id;

    @added(ApiVersions::v2)
    long userId;
};
";
    let spine = spine_of(text).expect("versioned");
    let cell = spine
        .correspondence
        .iter()
        .find(|cell| cell.from == "v1" && cell.to == "v2")
        .expect("the coincidence joins on the shared wire name");
    assert_eq!(cell.kind, CellKind::NameShifted);
    assert_eq!(cell.evidence, Evidence::WireNameMatch);
    assert_eq!(cell.from_subject.as_deref(), Some("Pet::user_id"));
    assert_eq!(cell.to_subject.as_deref(), Some("Pet::userId"));
}

#[test]
fn operations_join_by_ident_not_by_rename() {
    // Renaming an operation replaces one wire fact with another: the old
    // ident leaves, the new one arrives. The operation's wire identity is
    // its ident, so no annotation can forge a continuation.
    let text = format!(
        "{TIMELINE}
interface Store {{
    @added(ApiVersions::v2025_01)
    @removed(ApiVersions::v2025_06)
    Item get(long id);

    @added(ApiVersions::v2025_06)
    Item fetch(long id);
}};

struct Item {{
    string sku;
}};
"
    );
    let spine = spine_of(&text).expect("versioned");
    let departed = spine
        .correspondence
        .iter()
        .find(|cell| cell.wire_name == "get")
        .expect("the old operation must leave");
    assert_eq!(departed.kind, CellKind::Removed);
    assert_eq!(departed.evidence, Evidence::IntervalEnd);
    assert_eq!(departed.from_subject.as_deref(), Some("Store::get"));
    assert_eq!(departed.to_subject, None);
    let arrived = spine
        .correspondence
        .iter()
        .find(|cell| cell.wire_name == "fetch" && cell.from == "v2025_01")
        .expect("the new operation must arrive on the same pair");
    assert_eq!(arrived.kind, CellKind::Added);
    assert_eq!(arrived.evidence, Evidence::IntervalStart);
    assert_eq!(arrived.from_subject, None);
    assert_eq!(arrived.to_subject.as_deref(), Some("Store::fetch"));
    assert!(
        !spine
            .correspondence
            .iter()
            .any(|cell| cell.kind == CellKind::Renamed),
        "an operation rename is never a rename fact"
    );
}

#[test]
fn gap_resume_counts_every_skipped_version() {
    let text = "
@version
enum ApiVersions {
    v1,
    v2,
    v3,
    v4,
};

struct Pet {
    @added(ApiVersions::v1)
    @removed(ApiVersions::v2)
    long age;

    @added(ApiVersions::v4)
    @rename(\"age\")
    long age2_;
};
";
    let spine = spine_of(text).expect("versioned");
    let cell = spine
        .correspondence
        .iter()
        .find(|cell| cell.from == "v3" && cell.to == "v4")
        .expect("the resume is a cell of the pair that brings it back");
    assert_eq!(cell.kind, CellKind::Added);
    assert_eq!(
        cell.evidence,
        Evidence::ResumesAfterGap {
            gap: 2,
            confirmed: true
        },
        "the gap counts v2 and v3, the versions without the wire name"
    );
}
#[test]
fn same_wire_names_in_different_containers_stay_independent() {
    let text = format!(
        "{TIMELINE}
struct Box {{
    @added(ApiVersions::v2025_01)
    @removed(ApiVersions::v2025_06)
    long id;

    @added(ApiVersions::v2025_06)
    @rename(\"id\")
    long key_;
}};

struct Crate {{
    long id;
}};
"
    );
    let spine = spine_of(&text).expect("versioned");
    let box_cell = spine
        .correspondence
        .iter()
        .find(|cell| cell.container == "Box" && cell.wire_name == "id")
        .expect("Box renames its id");
    assert_eq!(box_cell.kind, CellKind::Renamed);
    assert_eq!(box_cell.from_subject.as_deref(), Some("Box::id"));
    assert_eq!(box_cell.to_subject.as_deref(), Some("Box::key_"));
    let crate_cell = spine
        .correspondence
        .iter()
        .find(|cell| cell.container == "Crate" && cell.wire_name == "id")
        .expect("Crate keeps its id");
    assert_eq!(crate_cell.kind, CellKind::Mechanical);
    assert_eq!(crate_cell.from_subject.as_deref(), Some("Crate::id"));
}
