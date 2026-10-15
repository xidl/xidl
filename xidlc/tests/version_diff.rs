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

    let out = run_capture(
        &workspace,
        &["xidlc", "version", "diff", &contract_name(&contract)],
    );
    assert!(out.contains("diff 2025-06 (v2) → 2025-12 (v3)"), "{out}");
}

#[test]
fn diff_text_reports_each_non_mechanical_fact() {
    let workspace = Workspace::new("diff-text");
    let contract = workspace.contract_path("catalog", DIFF_CONTRACT);

    let out = run_capture(
        &workspace,
        &[
            "xidlc",
            "version",
            "diff",
            "--from",
            "v1",
            "--to",
            "v2",
            &contract_name(&contract),
        ],
    );
    assert!(out.contains("diff 2025-01 (v1) → 2025-06 (v2)"), "{out}");
    assert!(
        out.contains("added         - → Item::note  [interval-start]"),
        "{out}"
    );
    assert!(
        out.contains("removed       Item::weight → -  [interval-end]"),
        "{out}"
    );
    assert!(
        out.contains("removed       Item::tag → -  [interval-end]"),
        "{out}"
    );
    assert!(
        out.contains("renamed       Item::size → Item::size2_  [explicit-rename]"),
        "{out}"
    );
    assert!(out.contains("mechanical cells: 1"), "{out}");
    assert!(
        out.contains("summary: 1 added, 2 removed, 1 renamed, 0 review"),
        "{out}"
    );
}

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

    let out = run_capture(
        &workspace,
        &["xidlc", "version", "diff", &contract_name(&contract)],
    );
    assert!(
        out.contains("name-shifted  Profile::user_id → Profile::userId  [wire-name-match]"),
        "{out}"
    );
    assert!(out.contains("review before adapters rely on it"), "{out}");
    assert!(out.contains("1 review"), "{out}");
}

#[test]
fn diff_json_carries_the_same_cells_as_the_text() {
    let workspace = Workspace::new("diff-json");
    let contract = workspace.contract_path("catalog", DIFF_CONTRACT);

    let out = run_capture(
        &workspace,
        &[
            "xidlc",
            "version",
            "diff",
            "--format",
            "json",
            "--from",
            "v1",
            "--to",
            "v2",
            &contract_name(&contract),
        ],
    );
    let report: serde_json::Value = serde_json::from_str(&out).expect("json diff parses");
    assert_eq!(report["from"]["ident"], "v1");
    assert_eq!(report["from"]["value"], "2025-01");
    assert_eq!(report["to"]["ident"], "v2");
    assert_eq!(report["selector"], serde_json::Value::Null);
    let summary = &report["summary"];
    assert_eq!(summary["added"], 1);
    assert_eq!(summary["removed"], 2);
    assert_eq!(summary["renamed"], 1);
    assert_eq!(summary["review"], 0);
    let cells = report["cells"].as_array().expect("cells array");
    assert_eq!(cells.len(), 5);
}

#[test]
fn diff_json_reports_the_selector_when_declared() {
    let workspace = Workspace::new("diff-json-selector");
    let contract = workspace.contract_path(
        "pinned",
        r#"
#pragma xidlc package pinned

@version
@version_selector(header = "Api-Version")
enum ApiVersions {
    @rename("2025-01") v1,
    @rename("2025-06") v2,
};

struct Item {
    string sku;
};
"#,
    );

    let out = run_capture(
        &workspace,
        &[
            "xidlc",
            "version",
            "diff",
            "--format",
            "json",
            &contract_name(&contract),
        ],
    );
    let report: serde_json::Value = serde_json::from_str(&out).expect("json diff parses");
    assert_eq!(report["selector"]["location"], "header");
    assert_eq!(report["selector"]["name"], "Api-Version");
}

#[test]
fn diff_resolves_display_values() {
    let workspace = Workspace::new("diff-values");
    let contract = workspace.contract_path("catalog", DIFF_CONTRACT);

    let out = run_capture(
        &workspace,
        &[
            "xidlc",
            "version",
            "diff",
            "--from",
            "2025-01",
            "--to",
            "2025-06",
            &contract_name(&contract),
        ],
    );
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

/// Runs the built `xidlc` binary inside the workspace and returns its stdout,
/// failing on a non-zero exit. `args` includes the program name, like
/// [`Workspace::run`].
fn run_capture(workspace: &Workspace, args: &[&str]) -> String {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_xidlc"))
        .args(&args[1..])
        .current_dir(&workspace.dir)
        .output()
        .expect("run the xidlc binary");
    assert!(
        output.status.success(),
        "xidlc failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("stdout is utf-8")
}

/// The workspace-relative path of a contract written by the harness.
fn contract_name(contract: &std::path::Path) -> String {
    contract
        .file_name()
        .and_then(|name| name.to_str())
        .expect("utf-8 file name")
        .to_string()
}
