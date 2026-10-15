#![cfg(feature = "cli")]

//! End-to-end coverage for `xidlc version manifest`: the spine artifact's
//! JSON shape, the selector and unversioned edges, and the file errors a CI
//! run hits first.

mod common;

use common::Workspace;

const VERSIONED: &str = r#"
#pragma xidlc package catalog

@version
enum ApiVersions {
    @rename("2025-01") v1,
    @rename("2025-06") v2,
};

struct Item {
    string sku;
    @added(ApiVersions::v2)
    @optional string note;
};

interface Store {
    @get(path = "/items/{id}")
    Item get(@path string id);
};
"#;

#[test]
fn manifest_carries_the_full_spine_shape() {
    let workspace = Workspace::new("manifest-shape");
    let contract = workspace.contract_path("catalog", VERSIONED);

    let out = workspace
        .run(&[
            "xidlc",
            "version",
            "manifest",
            contract.to_str().expect("utf-8"),
        ])
        .expect("xidlc should succeed");
    let report: serde_json::Value = serde_json::from_str(&out).expect("one manifest document");
    assert_eq!(report["file"], contract.to_str().expect("utf-8"));
    assert_eq!(report["spine"]["selector"], serde_json::Value::Null);

    let timeline = report["spine"]["timeline"].as_array().expect("timeline");
    assert_eq!(timeline.len(), 2);
    assert_eq!(timeline[0]["ident"], "v1");
    assert_eq!(timeline[0]["value"], "2025-01");
    assert_eq!(timeline[1]["ident"], "v2");
    assert_eq!(timeline[1]["value"], "2025-06");

    let surfaces = report["spine"]["surfaces"].as_array().expect("surfaces");
    assert_eq!(surfaces.len(), 2);
    for surface in surfaces {
        assert!(
            surface["digest"]
                .as_str()
                .expect("digest")
                .starts_with("md5:"),
            "digests carry their scheme: {surface}"
        );
    }
    assert_eq!(
        surfaces[0]["entries"], 2,
        "v1 carries the field and the operation"
    );
    assert_eq!(surfaces[1]["entries"], 3, "v2 adds note");

    let cells = report["spine"]["correspondence"]
        .as_array()
        .expect("correspondence");
    assert_eq!(
        cells.len(),
        3,
        "sku and the operation carry over, note arrives"
    );
    let added = cells
        .iter()
        .find(|cell| cell["kind"] == "added")
        .expect("note arrives");
    assert_eq!(added["wire_name"], "note");
    assert_eq!(added["evidence"], "interval-start");
    assert_eq!(added["from_subject"], serde_json::Value::Null);
    assert_eq!(added["to_subject"], "Item::note");
}

#[test]
fn manifest_declares_the_selector_when_pinned() {
    let workspace = Workspace::new("manifest-selector");
    let contract = workspace.contract_path(
        "pinned",
        r#"
#pragma xidlc package pinned

@version
@version_selector(query = "api-version")
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
            "manifest",
            contract.to_str().expect("utf-8"),
        ])
        .expect("xidlc should succeed");
    let report: serde_json::Value = serde_json::from_str(&out).expect("one manifest document");
    assert_eq!(report["spine"]["selector"]["location"], "query");
    assert_eq!(report["spine"]["selector"]["name"], "api-version");
}

#[test]
fn unversioned_contracts_manifest_without_a_spine() {
    let workspace = Workspace::new("manifest-plain");
    let contract = workspace.contract_path("plain", "struct Pet { long id; };");

    let out = workspace
        .run(&[
            "xidlc",
            "version",
            "manifest",
            contract.to_str().expect("utf-8"),
        ])
        .expect("xidlc should succeed");
    let report: serde_json::Value = serde_json::from_str(&out).expect("one manifest document");
    assert_eq!(report["spine"], serde_json::Value::Null);
}

#[test]
fn manifest_fails_loudly_on_missing_or_broken_files() {
    let workspace = Workspace::new("manifest-errors");

    let error = workspace
        .run(&["xidlc", "version", "manifest", "missing.idl"])
        .expect_err("a missing contract cannot manifest");
    assert!(error.contains("cannot read"), "{error}");

    let broken = workspace.contract_path("broken", "struct Pet {");
    let error = workspace
        .run(&[
            "xidlc",
            "version",
            "manifest",
            broken.to_str().expect("utf-8"),
        ])
        .expect_err("a contract that does not parse has no spine");
    assert!(!error.is_empty(), "the parse error is reported");
}

#[test]
fn manifest_prints_one_document_per_file() {
    let workspace = Workspace::new("manifest-multi");
    let catalog = workspace.contract_path("catalog", VERSIONED);
    let depot = workspace.contract_path(
        "depot",
        r#"
#pragma xidlc package depot

@version
enum DepotVersions {
    @rename("2025-01") v1,
};

struct Box {
    string sku;
};
"#,
    );

    let out = workspace
        .run(&[
            "xidlc",
            "version",
            "manifest",
            catalog.to_str().expect("utf-8"),
            depot.to_str().expect("utf-8"),
        ])
        .expect("xidlc should succeed");
    let documents: Vec<serde_json::Value> = serde_json::Deserializer::from_str(&out)
        .into_iter::<serde_json::Value>()
        .collect::<Result<_, _>>()
        .expect("one JSON document per file");
    assert_eq!(documents.len(), 2);
    assert_eq!(documents[0]["file"], catalog.to_str().expect("utf-8"));
    assert_eq!(documents[1]["file"], depot.to_str().expect("utf-8"));
    assert_eq!(
        documents[1]["spine"]["surfaces"]
            .as_array()
            .expect("v1")
            .len(),
        1
    );
}

#[test]
fn manifest_with_no_files_prints_nothing() {
    let workspace = Workspace::new("manifest-empty");

    let out = workspace
        .run(&["xidlc", "version", "manifest"])
        .expect("xidlc should succeed");
    assert!(out.is_empty(), "no files, no documents: {out}");
}
