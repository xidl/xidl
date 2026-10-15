#![cfg(feature = "cli")]

//! End-to-end regression for the version-diff review view: pair selection,
//! ident/value resolution, and the text and JSON renderings.

mod common;

use common::Workspace;

const DIFF_CONTRACT: &str = r#"
#pragma xidlc package catalog

@version
enum ApiVersions {
    @rename("2025-01") v1,
    @rename("2025-06") v2,
    @rename("2025-12") v3,
};

struct Item {
    string sku;
    @removed(ApiVersions::v2)
    long weight;

    @added(ApiVersions::v2)
    @optional string note;

    @added(ApiVersions::v1)
    @removed(ApiVersions::v2)
    long size;

    @added(ApiVersions::v2)
    @rename("size")
    long size2_;

    @added(ApiVersions::v1)
    @removed(ApiVersions::v2)
    long tag;

    @added(ApiVersions::v3)
    long tag2_;
};
"#;

const SINGLE_VERSION_CONTRACT: &str = r#"
#pragma xidlc package single

@version
enum ApiVersions {
    @rename("2025-01") v1,
};

struct Item {
    string sku;
};
"#;

const AMBIGUOUS_CONTRACT: &str = r#"
#pragma xidlc package ambiguous

@version
enum ApiVersions {
    @rename("v2") v1,
    @rename("v1") v2,
};

struct Item {
    string sku;
};
"#;

#[test]
fn diff_defaults_to_the_newest_adjacent_pair() {
    let workspace = Workspace::new("diff-default");
    let contract = workspace.contract_path("catalog", DIFF_CONTRACT);

    let out = workspace
        .run(&[
            "xidlc",
            "version",
            "diff",
            contract.to_str().expect("utf-8"),
        ])
        .expect("xidlc should succeed");
    assert!(out.contains("diff 2025-06 (v2) → 2025-12 (v3)"), "{out}");
}

#[test]
fn diff_resolves_display_values() {
    let workspace = Workspace::new("diff-values");
    let contract = workspace.contract_path("catalog", DIFF_CONTRACT);

    let out = workspace
        .run(&[
            "xidlc",
            "version",
            "diff",
            "--from",
            "2025-01",
            "--to",
            "2025-06",
            contract.to_str().expect("utf-8"),
        ])
        .expect("xidlc should succeed");
    assert!(out.contains("diff 2025-01 (v1) → 2025-06 (v2)"), "{out}");
}

#[test]
fn diff_rejects_non_adjacent_pairs() {
    let workspace = Workspace::new("diff-adjacent");
    let contract = workspace.contract_path("catalog", DIFF_CONTRACT);

    let error = workspace
        .run(&[
            "xidlc",
            "version",
            "diff",
            "--from",
            "v1",
            "--to",
            "v3",
            contract.to_str().expect("utf-8"),
        ])
        .expect_err("the spine records adjacent pairs only");
    assert!(error.contains("adjacent"), "{error}");
}

#[test]
fn diff_rejects_unknown_versions() {
    let workspace = Workspace::new("diff-unknown");
    let contract = workspace.contract_path("catalog", DIFF_CONTRACT);

    let error = workspace
        .run(&[
            "xidlc",
            "version",
            "diff",
            "--from",
            "v9",
            contract.to_str().expect("utf-8"),
        ])
        .expect_err("an unknown version cannot diff");
    assert!(error.contains("is not declared"), "{error}");
}

#[test]
fn diff_needs_two_declared_versions() {
    let workspace = Workspace::new("diff-single");
    let contract = workspace.contract_path("single", SINGLE_VERSION_CONTRACT);

    let error = workspace
        .run(&[
            "xidlc",
            "version",
            "diff",
            contract.to_str().expect("utf-8"),
        ])
        .expect_err("a single version has no pair");
    assert!(error.contains("at least two"), "{error}");
}

#[test]
fn diff_rejects_ambiguous_version_names() {
    let workspace = Workspace::new("diff-ambiguous");
    let contract = workspace.contract_path("ambiguous", AMBIGUOUS_CONTRACT);

    let error = workspace
        .run(&[
            "xidlc",
            "version",
            "diff",
            "--from",
            "v1",
            contract.to_str().expect("utf-8"),
        ])
        .expect_err("a name matching both an ident and a value is ambiguous");
    assert!(error.contains("matches both"), "{error}");
}

#[test]
fn diff_rejects_running_backwards() {
    let workspace = Workspace::new("diff-backwards");
    let contract = workspace.contract_path("catalog", DIFF_CONTRACT);

    let error = workspace
        .run(&[
            "xidlc",
            "version",
            "diff",
            "--from",
            "v2",
            "--to",
            "v1",
            contract.to_str().expect("utf-8"),
        ])
        .expect_err("the timeline runs forward only");
    assert!(error.contains("runs forward in the timeline"), "{error}");
    assert!(error.contains("v1 is declared before v2"), "{error}");
}

#[test]
fn diff_rejects_naming_one_version_twice() {
    let workspace = Workspace::new("diff-twice");
    let contract = workspace.contract_path("catalog", DIFF_CONTRACT);

    // `v1` and `2025-01` are the same member; naming it as both endpoints is
    // the confusion the pair resolver must catch before diffing.
    let error = workspace
        .run(&[
            "xidlc",
            "version",
            "diff",
            "--from",
            "v1",
            "--to",
            "2025-01",
            contract.to_str().expect("utf-8"),
        ])
        .expect_err("both endpoints are one version");
    assert!(error.contains("names one version twice"), "{error}");
}

#[test]
fn diff_skips_unversioned_contracts() {
    let workspace = Workspace::new("diff-plain");
    let contract = workspace.contract_path("plain", "struct Pet { long id; };");

    let out = workspace
        .run(&[
            "xidlc",
            "version",
            "diff",
            contract.to_str().expect("utf-8"),
        ])
        .expect("xidlc should succeed");
    assert!(out.is_empty(), "nothing to review: {out}");
}

#[test]
fn diff_with_two_versions_defaults_to_that_pair() {
    let workspace = Workspace::new("diff-two");
    let contract = workspace.contract_path(
        "pair",
        r#"
#pragma xidlc package pair

@version
enum ApiVersions {
    @rename("2025-01") v1,
    @rename("2025-06") v2,
};

struct Item {
    string sku;
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
    assert!(out.contains("diff 2025-01 (v1) → 2025-06 (v2)"), "{out}");
}

#[test]
fn diff_is_case_sensitive_about_version_names() {
    let workspace = Workspace::new("diff-case");
    let contract = workspace.contract_path("catalog", DIFF_CONTRACT);

    let error = workspace
        .run(&[
            "xidlc",
            "version",
            "diff",
            "--from",
            "V1",
            contract.to_str().expect("utf-8"),
        ])
        .expect_err("member idents are matched exactly");
    assert!(error.contains("version V1 is not declared"), "{error}");
    assert!(
        error.contains("declared: v1, v2, v3"),
        "the error names the alternatives: {error}"
    );
}
