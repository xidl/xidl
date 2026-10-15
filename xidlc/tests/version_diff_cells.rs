#![cfg(feature = "cli")]

//! End-to-end regression for the diff renderings: the text report lines,
//! the JSON schema with its audit evidence, and the pinned report format.

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

#[test]
fn diff_text_reports_each_non_mechanical_fact() {
    let workspace = Workspace::new("diff-text");
    let contract = workspace.contract_path("catalog", DIFF_CONTRACT);

    let out = workspace
        .run(&[
            "xidlc",
            "version",
            "diff",
            "--from",
            "v1",
            "--to",
            "v2",
            contract.to_str().expect("utf-8"),
        ])
        .expect("xidlc should succeed");
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
fn diff_json_carries_the_same_cells_as_the_text() {
    let workspace = Workspace::new("diff-json");
    let contract = workspace.contract_path("catalog", DIFF_CONTRACT);

    let out = workspace
        .run(&[
            "xidlc",
            "version",
            "diff",
            "--format",
            "json",
            "--from",
            "v1",
            "--to",
            "v2",
            contract.to_str().expect("utf-8"),
        ])
        .expect("xidlc should succeed");
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

    let out = workspace
        .run(&[
            "xidlc",
            "version",
            "diff",
            "--format",
            "json",
            contract.to_str().expect("utf-8"),
        ])
        .expect("xidlc should succeed");
    let report: serde_json::Value = serde_json::from_str(&out).expect("json diff parses");
    assert_eq!(report["selector"]["location"], "header");
    assert_eq!(report["selector"]["name"], "Api-Version");
}

#[test]
fn diff_text_declares_the_selector_line() {
    let workspace = Workspace::new("diff-selector-line");
    let plain = workspace.contract_path("catalog", DIFF_CONTRACT);
    let pinned = workspace.contract_path(
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

    let out = workspace
        .run(&["xidlc", "version", "diff", plain.to_str().expect("utf-8")])
        .expect("xidlc should succeed");
    assert!(out.contains("selector: none\n"), "{out}");
    let out = workspace
        .run(&["xidlc", "version", "diff", pinned.to_str().expect("utf-8")])
        .expect("xidlc should succeed");
    assert!(out.contains("selector: header \"Api-Version\"\n"), "{out}");
}

#[test]
fn diff_json_cells_carry_the_audit_trail() {
    let workspace = Workspace::new("diff-audit");
    let contract = workspace.contract_path("catalog", DIFF_CONTRACT);

    let out = workspace
        .run(&[
            "xidlc",
            "version",
            "diff",
            "--format",
            "json",
            "--from",
            "v1",
            "--to",
            "v2",
            contract.to_str().expect("utf-8"),
        ])
        .expect("xidlc should succeed");
    let report: serde_json::Value = serde_json::from_str(&out).expect("json diff parses");
    assert!(
        report["from"]["digest"]
            .as_str()
            .expect("digest")
            .starts_with("md5:"),
        "{report}"
    );
    let cells = report["cells"].as_array().expect("cells array");
    for cell in cells {
        for field in [
            "from",
            "to",
            "from_subject",
            "to_subject",
            "container",
            "wire_name",
            "kind",
            "evidence",
        ] {
            assert!(
                cell.get(field).is_some(),
                "every cell carries {field}: {cell}"
            );
        }
    }
    let renamed = cells
        .iter()
        .find(|cell| cell["kind"] == "renamed")
        .expect("the rename");
    assert_eq!(renamed["evidence"], "explicit-rename");
    let added = cells
        .iter()
        .find(|cell| cell["kind"] == "added")
        .expect("the arrival");
    assert_eq!(added["from_subject"], serde_json::Value::Null);
    assert_eq!(added["evidence"], "interval-start");
    let removed = cells
        .iter()
        .find(|cell| cell["kind"] == "removed")
        .expect("the departure");
    assert_eq!(removed["to_subject"], serde_json::Value::Null);
    assert_eq!(removed["evidence"], "interval-end");
}

#[test]
fn the_diff_report_renders_stably() {
    // The review report is what humans gate a release on; the snapshot pins
    // its exact rendering, review marker included.
    let workspace = Workspace::new("diff-snapshot");
    let contract = workspace.contract_path(
        "report",
        r#"
#pragma xidlc package report

@version
@version_selector(header = "Api-Version")
enum ApiVersions {
    @rename("2025-01") v1,
    @rename("2025-06") v2,
};

@rename_all("camelCase")
struct Pet {
    long id;

    @added(ApiVersions::v2)
    @optional string note;

    @added(ApiVersions::v1)
    @removed(ApiVersions::v2)
    long weight;

    @added(ApiVersions::v1)
    @removed(ApiVersions::v2)
    long size;

    @added(ApiVersions::v2)
    @rename("size")
    long size2_;

    @added(ApiVersions::v1)
    @removed(ApiVersions::v2)
    long user_id;

    @added(ApiVersions::v2)
    long userId;
};

interface Store {
    @get(path = "/pets/{id}")
    Pet find(@path string id);
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
    insta::assert_snapshot!("version_diff_report", out);
}
