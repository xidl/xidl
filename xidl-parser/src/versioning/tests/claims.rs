use super::{lower_error, parse};

fn err(text: &str) -> String {
    lower_error(text).to_string()
}

#[test]
fn overlapping_wire_names_are_rejected_per_version() {
    let error = err("
@version
enum ApiVersions {
    @rename(\"2025-01\") v1,
    @rename(\"2025-06\") v2,
};

struct Pet {
    long tag;

    @rename(\"tag\")
    long label;
};
");
    assert!(
        error.contains("both serialize as \"tag\""),
        "unexpected error: {error}"
    );
    assert!(
        error.contains("in version 2025-01"),
        "the diagnostic names the first version where they coexist: {error}"
    );
    assert!(error.contains("inside Pet"), "{error}");
}

#[test]
fn disjoint_rename_intervals_are_accepted() {
    parse(
        "
@version
enum ApiVersions {
    @rename(\"2025-01\") v1,
    @rename(\"2025-06\") v2,
};

struct Pet {
    @added(ApiVersions::v1)
    @removed(ApiVersions::v2)
    long tag;

    @added(ApiVersions::v2)
    @rename(\"tag\")
    long tag2_;
};
",
    );
}

#[test]
fn unversioned_contracts_reject_duplicate_wire_names() {
    let error = err("
struct Pet {
    long size;

    @rename(\"size\")
    long size_;
};
");
    assert!(
        error.contains("both serialize as \"size\""),
        "unexpected error: {error}"
    );
    assert!(
        !error.contains("in version"),
        "an unversioned contract has no version to name: {error}"
    );
}

#[test]
fn unversioned_repeated_members_are_rejected() {
    let error = err("struct Pet { long size; long size; };");
    assert!(error.contains("serialize as"), "{error}");
}

#[test]
fn skipped_members_claim_no_wire_name() {
    // The skipped declaration never serializes, so its name stays free for a
    // later interval without tripping the overlap check.
    parse(
        "
@version
enum ApiVersions {
    @rename(\"2025-01\") v1,
    @rename(\"2025-06\") v2,
};

struct Pet {
    @skip
    @added(ApiVersions::v1)
    long note;

    @added(ApiVersions::v2)
    string note;
};
",
    );
}

#[test]
fn enum_and_union_members_are_validated() {
    let error = err("
enum Color {
    RED,

    @rename(\"RED\")
    CRIMSON,
};
");
    assert!(error.contains("both serialize as \"RED\""), "{error}");
    assert!(error.contains("inside Color"), "{error}");

    let error = err("
union Shape switch (long) {
    case 0: long area;

    case 1: @rename(\"area\") string label;
};
");
    assert!(error.contains("both serialize as \"area\""), "{error}");
    assert!(error.contains("inside Shape"), "{error}");
}
