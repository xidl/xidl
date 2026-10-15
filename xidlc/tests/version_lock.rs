#![cfg(feature = "cli")]

//! End-to-end regression for the lock file as an artifact: shared locks,
//! contract keying, corruption and tampering.

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
fn shared_lock_keeps_other_contracts_frozen() {
    let workspace = Workspace::new("shared");
    let shared = workspace.dir.join("shared.versions.lock.json");
    let petstore = workspace.contract_path("petstore", CONTRACT);
    let warehouse = workspace.contract_path(
        "warehouse",
        r#"
#pragma xidlc package warehouse

@version
enum WarehouseVersions {
    v1,
};

struct Box {
    string sku;
};

interface Warehouse {
    @get(path = "/boxes/{sku}")
    Box get(@path string sku);
};
"#,
    );

    for contract in [&petstore, &warehouse] {
        workspace
            .run(&[
                "xidlc",
                "version",
                "verify",
                "--lock",
                shared.to_str().expect("utf-8"),
                "--update",
                contract.to_str().expect("utf-8"),
            ])
            .expect("both contracts freeze into the shared lock");
    }

    // Freezing the second contract must not have erased the first.
    workspace
        .run(&[
            "xidlc",
            "version",
            "verify",
            "--lock",
            shared.to_str().expect("utf-8"),
            petstore.to_str().expect("utf-8"),
        ])
        .expect("petstore should still verify against the shared lock");
    workspace
        .run(&[
            "xidlc",
            "version",
            "verify",
            "--lock",
            shared.to_str().expect("utf-8"),
            warehouse.to_str().expect("utf-8"),
        ])
        .expect("warehouse should still verify against the shared lock");

    // Verifying petstore must not be fooled by warehouse's digests.
    let drifted = format!("{CONTRACT}\nstruct Shelf {{ string code; }};\n");
    fs::write(&petstore, drifted).expect("write drifted contract");
    let error = workspace
        .run(&[
            "xidlc",
            "version",
            "verify",
            "--lock",
            shared.to_str().expect("utf-8"),
            petstore.to_str().expect("utf-8"),
        ])
        .expect_err("petstore drift should be reported against its own entry");
    assert!(
        error.contains("not frozen yet") || error.contains("changed"),
        "{error}"
    );
}

#[test]
fn a_damaged_lock_fails_closed() {
    let workspace = Workspace::new("freeze-damaged");
    let contract = workspace.contract_path("petstore", CONTRACT);
    verify(&workspace, &contract, &["--update"]).expect("freeze the contract");
    let lock_path = workspace.dir.join("petstore.versions.lock.json");

    // A truncated lock is corruption, never an empty slate.
    fs::write(&lock_path, "{ not json").expect("write corrupt lock");
    assert!(
        verify(&workspace, &contract, &[]).is_err(),
        "verify must not read a corrupt lock as a clean slate"
    );
    assert!(
        verify(&workspace, &contract, &["--update"]).is_err(),
        "--update must not treat a corrupt lock as empty"
    );

    // A foreign schema is not ours to rewrite.
    let foreign = r#"{"schema":"someone.else/v1","contracts":{}}"#;
    fs::write(&lock_path, foreign).expect("write foreign lock");
    let error = verify(&workspace, &contract, &[]).expect_err("a foreign schema is not a lock");
    assert!(
        error.contains("is not a xidl.versions-lock/v1 lock file"),
        "{error}"
    );
    let error = verify(&workspace, &contract, &["--update"])
        .expect_err("--update refuses to rewrite a file that is not a lock");
    assert!(
        error.contains("is not a xidl.versions-lock/v1 lock file"),
        "{error}"
    );
    assert_eq!(
        fs::read_to_string(&lock_path).expect("reread lock"),
        foreign,
        "--update leaves the foreign file untouched"
    );

    // A lock whose contracts entry is not an object is corruption, for
    // reading and rewriting alike — never a misleading empty slate.
    fs::write(
        &lock_path,
        r#"{"schema":"xidl.versions-lock/v1","contracts":[]}"#,
    )
    .expect("write shapeless lock");
    let error = verify(&workspace, &contract, &[])
        .expect_err("a non-object contracts entry is not a readable lock");
    assert!(
        error.contains("does not parse as a xidl.versions-lock/v1 lock file"),
        "{error}"
    );
    let error = verify(&workspace, &contract, &["--update"])
        .expect_err("--update refuses to rewrite a lock it cannot read");
    assert!(
        error.contains("does not parse as a xidl.versions-lock/v1 lock file"),
        "{error}"
    );
    assert_eq!(
        fs::read_to_string(&lock_path).expect("reread lock"),
        r#"{"schema":"xidl.versions-lock/v1","contracts":[]}"#,
        "--update leaves the unreadable file untouched"
    );
}

#[test]
fn a_tampered_lock_fails_the_release_gate() {
    let workspace = Workspace::new("freeze-tamper");
    let contract = workspace.contract_path("petstore", CONTRACT);
    verify(&workspace, &contract, &["--update"]).expect("freeze the contract");
    let lock_path = workspace.dir.join("petstore.versions.lock.json");
    let mut lock: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&lock_path).expect("read lock"))
            .expect("the fresh lock parses");
    let genuine = lock["contracts"]["petstore"]["versions"]["v1"]["digest"]
        .as_str()
        .expect("frozen digest")
        .to_string();
    let mut forged = genuine.clone();
    forged.replace_range(
        genuine.len() - 1..,
        if genuine.ends_with('0') { "1" } else { "0" },
    );
    lock["contracts"]["petstore"]["versions"]["v1"]["digest"] = serde_json::json!(forged);
    fs::write(
        &lock_path,
        serde_json::to_string_pretty(&lock).expect("serialize"),
    )
    .expect("write tampered lock");

    let error =
        verify(&workspace, &contract, &[]).expect_err("a hand-edited digest must fail the gate");
    assert!(
        error.contains(&format!(
            "released version v1 changed: {forged} → {genuine}"
        )),
        "{error}"
    );
}

#[test]
fn same_stem_contracts_collide_in_a_shared_lock() {
    // Pinned, deliberate trap: lock entries are keyed by file stem, so two
    // contracts in different directories that share a stem overwrite each
    // other's entry in a shared lock without warning, and the first
    // contract's gate silently stops protecting anything. Fixing this needs
    // provenance in the lock schema — a maintainer decision.
    let workspace = Workspace::new("freeze-stems");
    let shared = workspace.dir.join("shared.versions.lock.json");
    let depot_a = workspace.dir.join("a");
    let depot_b = workspace.dir.join("b");
    fs::create_dir(&depot_a).expect("create a/");
    fs::create_dir(&depot_b).expect("create b/");
    let first = depot_a.join("depot.idl");
    let second = depot_b.join("depot.idl");
    fs::write(&first, MINIMAL_CONTRACT).expect("write first depot");
    fs::write(&second, CONTRACT).expect("write second depot");
    let lock_arg = shared.to_str().expect("utf-8");

    verify(&workspace, &first, &["--lock", lock_arg, "--update"]).expect("the first depot freezes");
    verify(&workspace, &second, &["--lock", lock_arg, "--update"])
        .expect("the second depot freezes over the first entry");

    let error = verify(&workspace, &first, &["--lock", lock_arg])
        .expect_err("the first depot's entry was silently replaced");
    assert!(error.contains("release freeze violated"), "{error}");
}
