#![cfg(feature = "cli")]

//! End-to-end regression for what counts as frozen identity: the wire
//! version selector, the route bindings, the version wire values and the
//! declared timeline order.

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

const SELECTOR_CONTRACT: &str = r#"
#pragma xidlc package billing

@version
@version_selector(header = "Api-Version")
enum BillingVersions {
    @rename("2025-01") v1,
    @rename("2025-06") v2,
};

struct Invoice {
    string id;
};

interface Billing {
    @get(path = "/invoices/{id}")
    Invoice get(@path string id);
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
fn selector_drift_is_frozen_too() {
    let workspace = Workspace::new("selector");
    let contract = workspace.contract_path("billing", SELECTOR_CONTRACT);

    workspace
        .run(&[
            "xidlc",
            "version",
            "verify",
            contract.to_str().expect("utf-8"),
            "--update",
        ])
        .expect("the selector contract should freeze");
    let lock =
        fs::read_to_string(workspace.dir.join("billing.versions.lock.json")).expect("read lock");
    assert!(
        lock.contains("header:Api-Version"),
        "lock should freeze the selector: {lock}"
    );

    // Renaming the selector header is a breaking wire change for every
    // released version, so the freeze must catch it without a digest change.
    fs::write(
        &contract,
        SELECTOR_CONTRACT.replace(
            "@version_selector(header = \"Api-Version\")",
            "@version_selector(header = \"X-Billing-Version\")",
        ),
    )
    .expect("write renamed-selector contract");
    let error = workspace
        .run(&[
            "xidlc",
            "version",
            "verify",
            contract.to_str().expect("utf-8"),
        ])
        .expect_err("a renamed selector header is drift");
    assert!(error.contains("version selector changed"), "{error}");

    workspace
        .run(&[
            "xidlc",
            "version",
            "verify",
            contract.to_str().expect("utf-8"),
            "--update",
        ])
        .expect("intentional selector change should rewrite the lock");
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
fn route_rename_drifts_the_freeze() {
    let workspace = Workspace::new("freeze-routes");
    let contract = workspace.contract_path(
        "store",
        r#"
#pragma xidlc package store

@version
enum ApiVersions {
    @rename("2025-01") v1,
    @rename("2025-06") v2,
};

struct Item {
    string id;
};

interface Store {
    @get(path = "/things/{id}")
    Item get(@path string id);
};
"#,
    );

    workspace
        .run(&[
            "xidlc",
            "version",
            "verify",
            contract.to_str().expect("utf-8"),
            "--update",
        ])
        .expect("the original routes should freeze");

    // Moving a route is a breaking wire change for every client, even though
    // the operation ident, signature and types stay identical.
    fs::write(
        &contract,
        r#"
#pragma xidlc package store

@version
enum ApiVersions {
    @rename("2025-01") v1,
    @rename("2025-06") v2,
};

struct Item {
    string id;
};

interface Store {
    @get(path = "/items/{id}")
    Item get(@path string id);
};
"#,
    )
    .expect("write route-renamed contract");
    let error = workspace
        .run(&[
            "xidlc",
            "version",
            "verify",
            contract.to_str().expect("utf-8"),
        ])
        .expect_err("a route rename is release drift");
    assert!(error.contains("release freeze violated"), "{error}");
}

#[test]
fn a_selector_appearing_is_drift() {
    // A selector can only be introduced before the first release; after the
    // freeze it is a wire change for every released version.
    let workspace = Workspace::new("freeze-selector-late");
    let contract = workspace.contract_path("petstore", CONTRACT);
    verify(&workspace, &contract, &["--update"]).expect("freeze the selectorless contract");

    fs::write(
        &contract,
        CONTRACT.replace(
            "@version\nenum ApiVersions {",
            "@version\n@version_selector(header = \"Api-Version\")\nenum ApiVersions {",
        ),
    )
    .expect("write selector contract");
    let error = verify(&workspace, &contract, &[])
        .expect_err("introducing a selector changes every released version's wire");
    assert!(
        error.contains("version selector changed: none → header:Api-Version"),
        "{error}"
    );
}

#[test]
fn a_wire_value_change_is_drift() {
    // The version value is what clients send on the wire; retyping it breaks
    // every pinned caller even when no surface digest moves.
    let workspace = Workspace::new("freeze-value");
    let contract = workspace.contract_path("depot", MINIMAL_CONTRACT);
    verify(&workspace, &contract, &["--update"]).expect("freeze the contract");

    fs::write(
        &contract,
        MINIMAL_CONTRACT.replace("@rename(\"2025-06\") v2", "@rename(\"2025-07\") v2"),
    )
    .expect("write revalued contract");
    let error = verify(&workspace, &contract, &[]).expect_err("a changed wire value is drift");
    assert!(
        error.contains("released version v2 changed its wire value: 2025-06 → 2025-07"),
        "{error}"
    );
    assert!(
        !error.contains("changed: md5:"),
        "the surface digest is untouched: {error}"
    );
}

#[test]
fn a_reordered_timeline_is_drift() {
    // The declared order is release semantics: it decides which pairs are
    // adjacent, so a silent reorder must not slip past the gate.
    let workspace = Workspace::new("freeze-order");
    let contract = workspace.contract_path("depot", MINIMAL_CONTRACT);
    verify(&workspace, &contract, &["--update"]).expect("freeze the contract");

    fs::write(
        &contract,
        MINIMAL_CONTRACT.replace(
            "    @rename(\"2025-01\") v1,\n    @rename(\"2025-06\") v2,",
            "    @rename(\"2025-06\") v2,\n    @rename(\"2025-01\") v1,",
        ),
    )
    .expect("write reordered contract");
    let error = verify(&workspace, &contract, &[]).expect_err("a reordered timeline is drift");
    assert!(
        error.contains("version timeline changed: v1, v2 → v2, v1"),
        "{error}"
    );
}
