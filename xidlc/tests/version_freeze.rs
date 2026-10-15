#![cfg(feature = "cli")]

//! End-to-end regression for the release-freeze gate: normalized copies per
//! version value, every drift report line, the `--update` door and its
//! append-only history rules.

mod common;

use common::Workspace;
use std::fs;
use std::path::{Path, PathBuf};

/// A contract without lifecycle references, so members can come and go.
const DEPOT_CONTRACT: &str = r#"
#pragma xidlc package depot

@version
enum ApiVersions {
    @rename("2025-01") v1,
    @rename("2025-06") v2,
};

struct Box {
    string sku;
};

interface Depot {
    @get(path = "/boxes/{sku}")
    Box get(@path string sku);
};
"#;

const CONTRACT: &str = r#"
#pragma xidlc package petstore

@version
enum ApiVersions {
    @rename("2025-01") v1,
    @rename("2025-06") v2,
};

struct Pet {
    string id;
    @added(ApiVersions::v2)
    @optional string note;
};

interface Pets {
    @get(path = "/pets/{id}")
    Pet get(@path string id);
};
"#;

/// Runs `xidlc version verify` on `contract` inside the workspace.
fn verify(workspace: &Workspace, contract: &Path, extra: &[&str]) -> Result<String, String> {
    let mut args = vec![
        "xidlc",
        "version",
        "verify",
        contract.to_str().expect("utf-8"),
    ];
    args.extend_from_slice(extra);
    workspace.run(&args)
}

fn versions_dir(workspace: &Workspace, contract: &str) -> PathBuf {
    workspace.dir.join(format!("{contract}.versions"))
}

#[test]
fn freeze_verify_update_cycle() {
    let workspace = Workspace::new("freeze-cycle");
    let contract = workspace.contract_path("petstore", CONTRACT);

    verify(&workspace, &contract, &["--update"]).expect("first update freezes both versions");
    let dir = versions_dir(&workspace, "petstore");
    assert!(dir.join("2025-01.json").exists(), "v1 copy by wire value");
    assert!(dir.join("2025-06.json").exists(), "v2 copy by wire value");
    assert!(dir.join("index.json").exists(), "release order manifest");

    verify(&workspace, &contract, &[]).expect("unchanged contract verifies");

    let rewritten = fs::read_to_string(dir.join("2025-01.json")).expect("read copy");
    verify(&workspace, &contract, &["--update"])
        .expect("re-freezing an unchanged contract is idempotent");
    assert_eq!(
        fs::read_to_string(dir.join("2025-01.json")).expect("read copy again"),
        rewritten,
        "normalized output is deterministic"
    );
}

#[test]
fn verify_without_frozen_copies_tells_how_to_create_them() {
    let workspace = Workspace::new("freeze-missing");
    let contract = workspace.contract_path("petstore", CONTRACT);

    let error = verify(&workspace, &contract, &[])
        .expect_err("verifying without copies must not silently pass");
    assert!(error.contains("no frozen versions"), "{error}");
    assert!(
        error.contains("xidlc version verify --update"),
        "the error names the door: {error}"
    );
}

#[test]
fn prose_edits_do_not_read_as_drift() {
    let workspace = Workspace::new("freeze-prose");
    let contract = workspace.contract_path("petstore", CONTRACT);
    verify(&workspace, &contract, &["--update"]).expect("freeze");

    let documented = format!("{CONTRACT}\n// a comment that changes nothing on the wire\n");
    fs::write(&contract, documented).expect("rewrite contract");

    verify(&workspace, &contract, &[]).expect("generated output is unchanged");
}

#[test]
fn released_version_drift_is_reported_structurally() {
    let workspace = Workspace::new("freeze-drift");
    let contract = workspace.contract_path("petstore", CONTRACT);
    verify(&workspace, &contract, &["--update"]).expect("freeze");

    let drifted = CONTRACT.replace(
        "    Pet get(@path string id);",
        "    Pet fetch(@path string id);",
    );
    fs::write(&contract, drifted).expect("rewrite contract");

    let error = verify(&workspace, &contract, &[])
        .expect_err("renaming a released operation changes the wire");
    assert!(error.contains("release freeze violated"), "{error}");
    assert!(
        error.contains("released version 2025-01 changed:"),
        "{error}"
    );
    assert!(
        error.contains("/paths//pets/{id}/get/operationId"),
        "{error}"
    );
}

#[test]
fn released_version_disappearing_fails() {
    let workspace = Workspace::new("freeze-gone");
    let contract = workspace.contract_path("depot", DEPOT_CONTRACT);
    verify(&workspace, &contract, &["--update"]).expect("freeze");

    let dropped = DEPOT_CONTRACT.replace("    @rename(\"2025-06\") v2,\n", "");
    fs::write(&contract, dropped).expect("rewrite contract");
    let error = verify(&workspace, &contract, &[])
        .expect_err("a released version must not disappear silently");
    assert!(
        error.contains("released version 2025-06 disappeared"),
        "{error}"
    );
}

#[test]
fn released_version_value_change_fails() {
    let workspace = Workspace::new("freeze-value");
    let contract = workspace.contract_path("petstore", CONTRACT);
    verify(&workspace, &contract, &["--update"]).expect("freeze");

    let renamed = CONTRACT.replace("@rename(\"2025-06\") v2", "@rename(\"2025-07\") v2");
    fs::write(&contract, renamed).expect("rewrite contract");
    let error = verify(&workspace, &contract, &[])
        .expect_err("changing a released version's wire value is a breaking edit");
    assert!(
        error.contains("released version 2025-06 disappeared"),
        "{error}"
    );
    assert!(error.contains("is not frozen yet"), "{error}");
}

#[test]
fn appending_a_version_requires_the_update_door() {
    let workspace = Workspace::new("freeze-append");
    let contract = workspace.contract_path("petstore", CONTRACT);
    verify(&workspace, &contract, &["--update"]).expect("freeze");

    let appended = CONTRACT.replace(
        "    @rename(\"2025-06\") v2,",
        "    @rename(\"2025-06\") v2,\n    @rename(\"2026-01\") v3,",
    );
    fs::write(&contract, appended).expect("rewrite contract");
    let error =
        verify(&workspace, &contract, &[]).expect_err("an unfrozen version fails verification");
    assert!(error.contains("new version v3 (2026-01)"), "{error}");

    verify(&workspace, &contract, &["--update"]).expect("an appended version is a legal release");
    assert!(
        versions_dir(&workspace, "petstore")
            .join("2026-01.json")
            .exists()
    );
    verify(&workspace, &contract, &[]).expect("verify passes after the door");
}

#[test]
fn inserting_before_a_released_version_is_refused() {
    let workspace = Workspace::new("freeze-insert");
    let contract = workspace.contract_path("petstore", CONTRACT);
    verify(&workspace, &contract, &["--update"]).expect("freeze");

    let inserted = CONTRACT.replace(
        "    @rename(\"2025-01\") v1,",
        "    @rename(\"2024-12\") v0,\n    @rename(\"2025-01\") v1,",
    );
    fs::write(&contract, inserted).expect("rewrite contract");
    let error = verify(&workspace, &contract, &["--update"])
        .expect_err("history cannot be rewritten by inserting before a release");
    assert!(
        error.contains("is inserted before released version"),
        "{error}"
    );
    assert!(
        error.contains("append-only"),
        "the error states the rule: {error}"
    );
}

#[test]
fn reordering_released_versions_is_refused() {
    let workspace = Workspace::new("freeze-reorder");
    let contract = workspace.contract_path("petstore", CONTRACT);
    verify(&workspace, &contract, &["--update"]).expect("freeze");

    let reordered = CONTRACT.replace(
        "    @rename(\"2025-01\") v1,\n    @rename(\"2025-06\") v2,",
        "    @rename(\"2025-06\") v2,\n    @rename(\"2025-01\") v1,",
    );
    fs::write(&contract, reordered).expect("rewrite contract");
    let error = verify(&workspace, &contract, &["--update"])
        .expect_err("reordering released versions rewrites history");
    assert!(error.contains("were reordered"), "{error}");
}
