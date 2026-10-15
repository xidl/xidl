#![cfg(feature = "cli")]

//! End-to-end regression for the release-freeze gate lifecycle: freeze,
//! verify, every drift report line, and the `--update` door.

mod common;

use common::Workspace;
use std::fs;

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

/// A contract whose version members carry no lifecycle references, so the
/// enum itself can gain or lose members without breaking the rest.
const MINIMAL_CONTRACT: &str = r#"
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

/// Runs `xidlc version verify` on `contract` in the workspace.
fn verify(
    workspace: &Workspace,
    contract: &std::path::Path,
    extra: &[&str],
) -> Result<String, String> {
    let mut args = vec![
        "xidlc",
        "version",
        "verify",
        contract.to_str().expect("utf-8"),
    ];
    args.extend_from_slice(extra);
    workspace.run(&args)
}

#[test]
fn lock_pass_drift_update_cycle() {
    let workspace = Workspace::new("cycle");
    let contract = workspace.contract_path("petstore", CONTRACT);

    workspace
        .run(&[
            "xidlc",
            "version",
            "verify",
            contract.to_str().expect("utf-8"),
            "--update",
        ])
        .expect("first verify --update should create the lock");
    let lock = workspace.dir.join("petstore.versions.lock.json");
    assert!(lock.exists(), "lock file should exist beside the contract");

    workspace
        .run(&[
            "xidlc",
            "version",
            "verify",
            contract.to_str().expect("utf-8"),
        ])
        .expect("unchanged contract should verify");

    let drifted = format!("{CONTRACT}\nstruct Tag {{ string name; }};\n");
    fs::write(&contract, drifted).expect("write drifted contract");
    let error = workspace
        .run(&[
            "xidlc",
            "version",
            "verify",
            contract.to_str().expect("utf-8"),
        ])
        .expect_err("a new top-level type is drift for every released version");
    assert!(error.contains("release freeze violated"), "{error}");

    workspace
        .run(&[
            "xidlc",
            "version",
            "verify",
            contract.to_str().expect("utf-8"),
            "--update",
        ])
        .expect("intentional change should rewrite the lock");
    workspace
        .run(&[
            "xidlc",
            "version",
            "verify",
            contract.to_str().expect("utf-8"),
        ])
        .expect("verify should pass after the door");
}

#[test]
fn verify_without_a_lock_tells_how_to_create_one() {
    let workspace = Workspace::new("freeze-missing");
    let contract = workspace.contract_path("petstore", CONTRACT);

    let error = verify(&workspace, &contract, &[])
        .expect_err("verifying without a lock must not silently pass");
    assert!(error.contains("no lock file"), "{error}");
    assert!(
        error.contains("xidlc version verify --update"),
        "the error names the door: {error}"
    );
}

#[test]
fn verify_names_every_drift_line() {
    let workspace = Workspace::new("freeze-lines");
    let contract = workspace.contract_path("depot", MINIMAL_CONTRACT);
    verify(&workspace, &contract, &["--update"]).expect("freeze both versions");
    let lock_path = workspace.dir.join("depot.versions.lock.json");
    let lock: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&lock_path).expect("read lock"))
            .expect("the fresh lock parses");
    let frozen_v2 = lock["contracts"]["depot"]["versions"]["v2"]["digest"]
        .as_str()
        .expect("frozen digest");

    // A version joins the timeline without passing the freeze door.
    fs::write(
        &contract,
        MINIMAL_CONTRACT.replace(
            "@rename(\"2025-06\") v2,",
            "@rename(\"2025-06\") v2,\n    @rename(\"2025-12\") v3,",
        ),
    )
    .expect("write extended contract");
    let error =
        verify(&workspace, &contract, &[]).expect_err("an unreleased version is not frozen yet");
    assert!(
        error.contains("new version v3 is not frozen yet"),
        "{error}"
    );

    // A released version disappears from the contract.
    fs::write(
        &contract,
        MINIMAL_CONTRACT.replace("    @rename(\"2025-06\") v2,\n", ""),
    )
    .expect("write reduced contract");
    let error =
        verify(&workspace, &contract, &[]).expect_err("dropping a released version is drift");
    assert!(
        error.contains(&format!(
            "released version v2 missing from the contract ({frozen_v2})"
        )),
        "{error}"
    );
}

#[test]
fn verify_reports_a_changed_surface_with_both_digests() {
    let workspace = Workspace::new("freeze-changed");
    let contract = workspace.contract_path("petstore", CONTRACT);
    verify(&workspace, &contract, &["--update"]).expect("freeze the contract");
    let lock_path = workspace.dir.join("petstore.versions.lock.json");
    let lock: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&lock_path).expect("read lock"))
            .expect("the fresh lock parses");
    let frozen = lock["contracts"]["petstore"]["versions"]["v2"]["digest"]
        .as_str()
        .expect("frozen digest");

    fs::write(
        &contract,
        CONTRACT.replace("@optional string note;", "@optional long note;"),
    )
    .expect("write changed contract");
    let error = verify(&workspace, &contract, &[]).expect_err("a released version's surface moved");
    assert!(
        error.contains(&format!("released version v2 changed: {frozen} → md5:")),
        "the report shows both digests: {error}"
    );
    assert!(
        !error.contains("released version v1"),
        "v1 is untouched: {error}"
    );
}

#[test]
fn refreezing_an_unchanged_contract_rewrites_byte_for_byte() {
    let workspace = Workspace::new("freeze-idempotent");
    let contract = workspace.contract_path("petstore", CONTRACT);
    let out = workspace
        .run(&[
            "xidlc",
            "version",
            "verify",
            "--update",
            contract.to_str().expect("utf-8"),
        ])
        .expect("xidlc should succeed");
    assert!(
        out.contains(&format!(
            "lock updated: {}",
            workspace.dir.join("petstore.versions.lock.json").display()
        )),
        "the freeze names the lock it wrote: {out}"
    );
    let lock_path = workspace.dir.join("petstore.versions.lock.json");
    let first = fs::read_to_string(&lock_path).expect("read the fresh lock");

    verify(&workspace, &contract, &[]).expect("the fresh lock verifies");
    verify(&workspace, &contract, &["--update"]).expect("re-freezing is allowed");
    let second = fs::read_to_string(&lock_path).expect("reread the lock");
    assert_eq!(
        first, second,
        "re-freezing an unchanged contract must not churn the file VCS stores"
    );
}

#[test]
fn unversioned_contracts_are_skipped_without_a_lock() {
    // Pinned, deliberate semantics: `verify` gates versioned contracts only.
    // The documented flip side — a de-versioned or mistyped contract
    // silently verifies green — is why `manifest` exists for CI audits.
    let workspace = Workspace::new("freeze-plain");
    let contract = workspace.contract_path("plain", "struct Pet { long id; };");

    verify(&workspace, &contract, &[]).expect("nothing to freeze");
    verify(&workspace, &contract, &["--update"]).expect("nothing to freeze");
    assert!(
        !workspace.dir.join("plain.versions.lock.json").exists(),
        "no lock is created for an unversioned contract"
    );
}

#[test]
fn a_contract_that_cannot_build_a_spine_fails_the_gate() {
    // Semantic errors — not just syntax — must stop the freeze before any
    // lock is written.
    let workspace = Workspace::new("freeze-semantic");
    let contract = workspace.contract_path(
        "clash",
        r#"
#pragma xidlc package clash

@version
enum ApiVersions {
    @rename("2025-01") v1,
    @rename("2025-06") v2,
};

struct Pet {
    long tag;

    @rename("tag")
    long label;
};
"#,
    );

    let error = verify(&workspace, &contract, &["--update"])
        .expect_err("overlapping wire names must fail the gate");
    assert!(error.contains("both serialize as \"tag\""), "{error}");
    assert!(
        !workspace.dir.join("clash.versions.lock.json").exists(),
        "no lock is written for a contract without a spine"
    );
}
