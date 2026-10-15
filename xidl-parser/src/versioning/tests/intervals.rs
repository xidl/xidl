//! Coverage for the read-only interval API the adapter stage pairs from.

use super::*;
use crate::versioning::IntervalKind;

#[test]
fn intervals_cover_member_lifetimes() {
    let (_, record) = analyze(
        r#"
@version
enum Versions {
    @rename("1.0") v1,
    @rename("2.0") v2,
};

struct Pet {
    string id;
    @added(Versions::v2) @optional string note;
    @removed(Versions::v2) string color;
};
"#,
    );

    let spec = parse(
        r#"
@version
enum Versions {
    @rename("1.0") v1,
    @rename("2.0") v2,
};

struct Pet {
    string id;
    @added(Versions::v2) @optional string note;
    @removed(Versions::v2) string color;
};
"#,
    );
    let intervals = record.member_intervals(&spec).expect("intervals");

    let member = |path: &str| {
        intervals
            .iter()
            .find(|interval| interval.path == path)
            .unwrap_or_else(|| panic!("missing interval for {path}"))
            .clone()
    };

    let id = member("Pet::id");
    assert_eq!(id.kind, IntervalKind::Member);
    assert_eq!((id.from, id.until), (0, 2), "always-present member");

    let note = member("Pet::note");
    assert_eq!((note.from, note.until), (1, 2), "added in v2");

    let color = member("Pet::color");
    assert_eq!((color.from, color.until), (0, 1), "removed at v2");

    let pet = member("Pet");
    assert_eq!(pet.kind, IntervalKind::Type);
    assert_eq!((pet.from, pet.until), (0, 2));

    assert!(
        !intervals.iter().any(|interval| interval.path == "Versions::v1"),
        "the version enum's own timeline is not reported"
    );
}

#[test]
fn operations_and_parameters_carry_intervals() {
    let (_, record) = analyze(
        r#"
@version
enum Versions {
    v1,
    v2,
};

struct Box {
    string sku;
};

interface Depot {
    @get(path = "/boxes/{sku}")
    Box get(@path string sku);
    @added(Versions::v2) @get(path = "/boxes")
    sequence<Box> list();
};
"#,
    );

    let spec = parse(
        r#"
@version
enum Versions {
    v1,
    v2,
};

struct Box {
    string sku;
};

interface Depot {
    @get(path = "/boxes/{sku}")
    Box get(@path string sku);
    @added(Versions::v2) @get(path = "/boxes")
    sequence<Box> list();
};
"#,
    );
    let intervals = record.member_intervals(&spec).expect("intervals");

    let list = intervals
        .iter()
        .find(|interval| interval.path == "Depot::list")
        .expect("operation interval");
    assert_eq!(list.kind, IntervalKind::Operation);
    assert_eq!((list.from, list.until), (1, 2));

    let sku = intervals
        .iter()
        .find(|interval| interval.path == "Depot::get::sku")
        .expect("parameter interval");
    assert_eq!(sku.kind, IntervalKind::Parameter);
    assert_eq!((sku.from, sku.until), (0, 2));
}

#[test]
fn unversioned_specifications_have_no_record_to_read_from() {
    let spec = parse("struct Pet { string id; };");
    let record = crate::versioning::VersioningRecord::analyze(&spec).expect("analysis");
    assert!(record.is_none(), "an unversioned specification has no record");
}

#[test]
fn malformed_lifecycle_annotations_fail_the_read() {
    let error = lower_error(
        r#"
@added(Versions::v1)
struct Pet {
    string id;
};

struct Widget {
    string name;
};
"#,
    );
    assert!(error.to_string().contains("@version enum"), "{error}");
}
