use crate::hir::Specification;
use crate::versioning::{CellKind, Evidence, Spine};

fn spine_of(text: &str) -> Option<Spine> {
    let typed = crate::parser::parser_text(text).expect("IDL should parse");
    Specification::versioning_spine(typed).expect("spine should build")
}

fn spine_err(text: &str) -> String {
    let typed = crate::parser::parser_text(text).expect("IDL should parse");
    Specification::versioning_spine(typed)
        .expect_err("spine should fail")
        .to_string()
}

const TIMELINE: &str = "
@version
enum ApiVersions {
    @rename(\"2025-01\") v2025_01,
    @rename(\"2025-06\") v2025_06,
    @rename(\"2026-01\") v2026_01,
};
";

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
    let optional_cell = spine
        .correspondence
        .iter()
        .find(|cell| cell.wire_name == "weight")
        .expect("optional cell");
    assert_eq!(optional_cell.kind, CellKind::Optionalized);
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
fn digests_are_stable_and_surface_sensitive() {
    let text = format!(
        "{TIMELINE}
struct Pet {{
    long id;
}};
"
    );
    let first = spine_of(&text).expect("versioned").surfaces;
    let second = spine_of(&text).expect("versioned").surfaces;
    assert_eq!(first, second);

    let changed = spine_of(&format!(
        "{TIMELINE}
struct Pet {{
    long id;

    @added(ApiVersions::v2025_06)
    long extra;
}};
"
    ))
    .expect("versioned")
    .surfaces;
    assert_eq!(changed[0].digest, first[0].digest, "v1 surface is intact");
    assert_ne!(changed[1].digest, first[1].digest, "v2 surface moved");
}

#[test]
fn unversioned_contracts_have_no_spine() {
    let spine = spine_of("struct Pet { long id; };");
    assert!(spine.is_none());
}
