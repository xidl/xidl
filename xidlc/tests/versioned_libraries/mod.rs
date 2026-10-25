#![cfg(all(
    feature = "gen-typescript-rest-adapters",
    feature = "gen-rust-axum-adapters",
    feature = "gen-openapi"
))]

mod fixture;

use fixture::Libraries;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;

#[test]
fn split_libraries_preserve_all_targets_and_selected_version_contracts() {
    let libraries = Libraries::new();
    let manifest = libraries.path("service/xidl.toml");
    // Explicit files must keep unrelated drafts out of the compilation units.
    fs::write(libraries.path("service/draft.idl"), "unfinished draft")
        .expect("write excluded draft");
    for target in [
        "rust-axum",
        "typescript-rest",
        "openapi",
        "rust-axum-adapters",
        "typescript-rest-adapters",
    ] {
        assert_eq!(
            libraries.generate(target, &manifest, None),
            libraries.generate(target, &Libraries::flat_contract(), None),
            "splitting modules must preserve every {target} artifact byte for byte"
        );
        if !target.ends_with("adapters") {
            for version in ["v1", "2026-04-01", "v3"] {
                assert_eq!(
                    libraries.generate(target, &manifest, Some(version)),
                    libraries.generate(target, &Libraries::flat_contract(), Some(version)),
                    "independent {target} {version} selection must survive package mounts"
                );
            }
        }
    }
}

#[test]
fn imported_readers_keep_model_bridges_and_versioned_external_schemas() {
    let libraries = Libraries::new();
    let manifest = libraries.path("readers/xidl.toml");
    let typescript = libraries.generate("typescript-rest", &manifest, None);
    let bridges: BTreeMap<_, _> = typescript
        .iter()
        .filter(|(name, _)| name.starts_with("old_reader") || name.starts_with("head_reader"))
        .map(|(name, content)| {
            let imports = content
                .lines()
                .filter(|line| {
                    line.contains("import * as")
                        || line.contains("export { versioned_inheritance }")
                })
                .collect::<Vec<_>>();
            (name, imports)
        })
        .collect();
    insta::assert_json_snapshot!("imported_reader_model_bridges", bridges);

    let documents = libraries.generate("openapi", &manifest, None);
    assert_eq!(
        documents.len(),
        5,
        "two readers share one three-version unit"
    );
    for (reader, version) in [("old_reader", "v1"), ("head_reader", "v3")] {
        let document: Value = serde_json::from_str(&documents[&format!("openapi_{reader}.json")])
            .expect("reader OpenAPI");
        let reference = document["paths"]["/tenants/{tenant}/addresses/{id}"]["get"]["responses"]
            ["200"]["content"]["application/json"]["schema"]["$ref"]
            .as_str()
            .expect("cross-unit response reference");
        let (file, pointer) = reference
            .split_once('#')
            .expect("external schema reference");
        assert_eq!(
            file,
            format!("openapi_versioned_inheritance.{version}.json")
        );
        let imported: Value = serde_json::from_str(&documents[file]).expect("imported OpenAPI");
        assert!(
            imported.pointer(pointer).is_some(),
            "{reference} must resolve"
        );
    }
}

#[test]
fn editable_path_dependencies_change_head_without_changing_old_contracts() {
    let libraries = Libraries::new();
    let manifest = libraries.path("service/xidl.toml");
    let before = libraries.generate("openapi", &manifest, None);
    let identity = libraries.path("identity/models/identity.idl");
    let source = fs::read_to_string(&identity).expect("identity source");
    fs::write(
        identity,
        source.replace(
            "struct Identity : Anchor {",
            "struct Identity : Anchor {\n    @added(Versions::v3) @optional string carrier_hint;",
        ),
    )
    .expect("edit the unpublished Head contract directly");
    let after = libraries.generate("openapi", &manifest, None);
    for version in ["v1", "v2"] {
        let file = format!("openapi_versioned_inheritance.{version}.json");
        assert_eq!(
            before[&file], after[&file],
            "published {version} stays frozen"
        );
    }
    let head = "openapi_versioned_inheritance.v3.json";
    assert!(!before[head].contains("carrierHint"));
    assert!(
        after[head].contains("carrierHint"),
        "path mounts read live source"
    );
}

#[test]
fn missing_nested_sources_and_mounts_name_the_failed_dependency() {
    let libraries = Libraries::new();
    let identity = libraries.path("identity/models/identity.idl");
    fs::remove_file(&identity).expect("remove nested identity source");
    let output = libraries.path("missing-include");
    let error = libraries.generate_error("openapi", &libraries.path("service/xidl.toml"), &output);
    assert!(error.contains("models/identity.idl"), "{error}");

    fs::remove_dir_all(libraries.path("identity")).expect("remove mounted identity directory");
    let error = libraries.generate_error("openapi", &libraries.path("service"), &output);
    assert!(error.contains("contracts/identity"), "{error}");
    assert!(error.contains("../identity"), "{error}");
}

#[test]
fn invalid_imported_versions_fail_before_any_unit_is_emitted() {
    let libraries = Libraries::new();
    let reader = libraries.path("readers/old_reader.idl");
    let source = fs::read_to_string(&reader).expect("reader source");
    fs::write(
        reader,
        source.replace("@v1", "@v99").replace("::v1::", "::v99::"),
    )
    .expect("select an undeclared imported version");
    let output = libraries.path("invalid-import");
    let error = libraries.generate_error(
        "typescript-rest",
        &libraries.path("readers/xidl.toml"),
        &output,
    );
    assert!(error.contains("v99"), "{error}");
    assert!(error.contains("v1, v2, v3"), "{error}");
    assert_eq!(fs::read_dir(output).expect("output directory").count(), 0);
}
