#![cfg(feature = "cli")]

//! End-to-end regression for the release-freeze gate: lock, pass, drift, and
//! the `--update` door, plus the shared-lock contract keying.

use clap::Parser as _;
use std::fs;
use std::path::PathBuf;
use std::process;
use xidlc::cli::Cli;

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

struct Workspace {
    dir: PathBuf,
}

impl Workspace {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("xidl-freeze-{}-{}", process::id(), name));
        fs::create_dir_all(&dir).expect("create workspace");
        Self { dir }
    }

    fn contract_path(&self, name: &str, source: &str) -> PathBuf {
        let path = self.dir.join(format!("{name}.idl"));
        fs::write(&path, source).expect("write contract");
        path
    }

    fn run(&self, args: &[&str]) -> Result<(), String> {
        let cli = Cli::try_parse_from(args).expect("parse version args");
        cli.run().map_err(|error| error.to_string())
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
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
