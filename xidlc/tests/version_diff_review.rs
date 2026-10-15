#![cfg(feature = "cli")]

//! End-to-end regression for the review semantics of `xidlc version diff`:
//! which carries are trusted, which need human review, and why.

mod common;

use common::Workspace;

#[test]
fn diff_marks_wire_coincidences_for_review() {
    let workspace = Workspace::new("diff-review");
    let contract = workspace.contract_path(
        "profiles",
        r#"
#pragma xidlc package profiles

@version
enum ApiVersions {
    @rename("2025-01") v1,
    @rename("2025-06") v2,
};

@rename_all("camelCase")
struct Profile {
    @added(ApiVersions::v1)
    @removed(ApiVersions::v2)
    long user_id;

    @added(ApiVersions::v2)
    long userId;
};
"#,
    );

    let out = workspace
        .run(&[
            "xidlc",
            "version",
            "diff",
            contract.to_str().expect("utf-8"),
        ])
        .expect("xidlc should succeed");
    assert!(
        out.contains("name-shifted  Profile::user_id → Profile::userId  [wire-name-match]"),
        "{out}"
    );
    assert!(out.contains("review before adapters rely on it"), "{out}");
    assert!(out.contains("1 review"), "{out}");
}

#[test]
fn diff_review_semantics_for_gap_resumes() {
    let workspace = Workspace::new("diff-resume");
    let contract = workspace.contract_path(
        "resumes",
        r#"
#pragma xidlc package resumes

@version
enum ApiVersions {
    @rename("2024-06") v1,
    @rename("2024-12") v2,
    @rename("2025-06") v3,
    @rename("2025-12") v4,
};

@rename_all("camelCase")
struct Job {
    long id;

    @added(ApiVersions::v1)
    @removed(ApiVersions::v2)
    long user_tag;

    @added(ApiVersions::v4)
    long userTag;

    @added(ApiVersions::v1)
    @removed(ApiVersions::v2)
    long flag;

    @added(ApiVersions::v4)
    @rename("flag")
    long flag2_;
};
"#,
    );

    let out = workspace
        .run(&[
            "xidlc",
            "version",
            "diff",
            contract.to_str().expect("utf-8"),
        ])
        .expect("xidlc should succeed");
    assert!(
        out.contains("added         - → Job::flag2_  [resume after 2, confirmed]"),
        "{out}"
    );
    assert!(
        out.contains("added         - → Job::userTag  [resume after 2, unconfirmed]"),
        "{out}"
    );
    assert_eq!(
        out.matches("^ review before adapters rely on it").count(),
        1,
        "only the rename-backed resume is trusted: {out}"
    );
    assert!(out.contains("mechanical cells: 1"), "{out}");
    assert!(out.contains("summary: 2 added, 1 review"), "{out}");

    let json = workspace
        .run(&[
            "xidlc",
            "version",
            "diff",
            "--format",
            "json",
            contract.to_str().expect("utf-8"),
        ])
        .expect("xidlc should succeed");
    let report: serde_json::Value = serde_json::from_str(&json).expect("json diff parses");
    assert_eq!(report["summary"]["added"], 2);
    assert_eq!(report["summary"]["review"], 1);
    let cells = report["cells"].as_array().expect("cells array");
    let confirmed = cells
        .iter()
        .find(|cell| cell["wire_name"] == "flag")
        .expect("the confirmed resume");
    assert_eq!(
        confirmed["evidence"],
        serde_json::json!({"resumes-after-gap": {"gap": 2, "confirmed": true}}),
        "{confirmed}"
    );
    let unconfirmed = cells
        .iter()
        .find(|cell| cell["wire_name"] == "userTag")
        .expect("the unconfirmed resume");
    assert_eq!(
        unconfirmed["evidence"],
        serde_json::json!({"resumes-after-gap": {"gap": 2, "confirmed": false}}),
        "{unconfirmed}"
    );
}

#[test]
fn diff_reports_type_and_optionality_changes() {
    let workspace = Workspace::new("diff-shapes");
    let contract = workspace.contract_path(
        "shapes",
        r#"
#pragma xidlc package shapes

@version
enum ApiVersions {
    @rename("2025-01") v1,
    @rename("2025-06") v2,
    @rename("2025-12") v3,
};

struct Pet {
    long id;

    @added(ApiVersions::v1)
    @removed(ApiVersions::v3)
    long age;

    @added(ApiVersions::v3)
    @rename("age")
    string age2_;

    @added(ApiVersions::v1)
    @removed(ApiVersions::v3)
    long weight;

    @added(ApiVersions::v3)
    @optional
    @rename("weight")
    long weight2_;
};
"#,
    );

    let out = workspace
        .run(&[
            "xidlc",
            "version",
            "diff",
            "--from",
            "v2",
            "--to",
            "v3",
            contract.to_str().expect("utf-8"),
        ])
        .expect("xidlc should succeed");
    assert!(
        out.contains("type-changed  Pet::age → Pet::age2_  [unchanged]"),
        "{out}"
    );
    assert!(
        out.contains("optionalized  Pet::weight → Pet::weight2_  [unchanged]"),
        "{out}"
    );
    assert!(
        out.contains("summary: 1 type-changed, 1 optionalized, 0 review"),
        "{out}"
    );

    let json = workspace
        .run(&[
            "xidlc",
            "version",
            "diff",
            "--format",
            "json",
            "--from",
            "v2",
            "--to",
            "v3",
            contract.to_str().expect("utf-8"),
        ])
        .expect("xidlc should succeed");
    let report: serde_json::Value = serde_json::from_str(&json).expect("json diff parses");
    assert_eq!(report["summary"]["type-changed"], 1);
    assert_eq!(report["summary"]["optionalized"], 1);
}
