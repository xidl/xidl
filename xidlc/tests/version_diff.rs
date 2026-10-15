#![cfg(feature = "cli")]

//! End-to-end regression for `xidlc version diff`: pair resolution and the
//! structural difference report between two versions.

mod common;

use common::Workspace;
use std::path::Path;

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

fn diff(workspace: &Workspace, contract: &Path, extra: &[&str]) -> Result<String, String> {
    let mut args = vec!["xidlc", "version", "diff"];
    args.extend_from_slice(extra);
    args.push(contract.to_str().expect("utf-8"));
    workspace.run(&args)
}

#[test]
fn diff_defaults_to_the_newest_adjacent_pair() {
    let workspace = Workspace::new("diff-default");
    let contract = workspace.contract_path("petstore", CONTRACT);

    let out = diff(&workspace, &contract, &[]).expect("default pair renders");
    assert!(out.contains("2025-01 → 2025-06"), "{out}");
    assert!(
        out.contains("+ /components/schemas/v2.Pet:") && out.contains("{\"properties\":{\"id\""),
        "the added member shows inside the v2 schema: {out}"
    );
}

#[test]
fn diff_resolves_members_by_ident_or_value() {
    let workspace = Workspace::new("diff-resolve");
    let contract = workspace.contract_path("petstore", CONTRACT);

    let by_ident = diff(&workspace, &contract, &["--from", "v1", "--to", "v2"])
        .expect("member idents resolve");
    let by_value = diff(
        &workspace,
        &contract,
        &["--from", "2025-01", "--to", "2025-06"],
    )
    .expect("display values resolve");
    assert_eq!(by_ident, by_value, "both spellings select the same pair");
}

#[test]
fn diff_rejects_unknown_versions_and_single_version_pairs() {
    let workspace = Workspace::new("diff-reject");
    let contract = workspace.contract_path("petstore", CONTRACT);

    let error = diff(&workspace, &contract, &["--from", "v1", "--to", "v9"])
        .expect_err("an unknown version cannot be diffed");
    assert!(error.contains("declares no version 'v9'"), "{error}");

    let error = diff(&workspace, &contract, &["--from", "v2", "--to", "v2"])
        .expect_err("a version cannot be diffed against itself");
    assert!(error.contains("two distinct versions"), "{error}");
}
