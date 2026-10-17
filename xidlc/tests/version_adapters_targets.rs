#![cfg(feature = "gen-typescript-rest-adapters")]

use std::collections::HashMap;
use std::path::Path;

fn render(source: &str, name: &str) -> Vec<(String, String)> {
    xidlc::driver::Generator::new("typescript-rest-adapters".into())
        .generate_from_idl(source, Path::new(name), HashMap::new())
        .expect("independent adapter generation")
        .into_iter()
        .map(|file| (file.path().into(), file.content().into()))
        .collect()
}

#[test]
fn independent_typescript_target_snapshots_wire_validation_and_business_decisions() {
    let source = include_str!("../../bdd/features/data/versioned_adapters_business.idl");
    let files = render(source, "business.idl");
    assert_eq!(files, render(source, "business.idl"));
    assert!(files[0].1.contains("optionalChangeDowncastToV1"));
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].0, "business.adapters.ts");
    insta::assert_snapshot!("independent_typescript_business_adapters", &files[0].1);
}

#[test]
fn single_version_target_has_valid_identity_entries_without_adjacent_conversions() {
    let files = render(
        "@version enum V { only }; @http(400) exception Invalid { string reason; }; interface Store { @post(path=\"/items\") string save(@body string value) raises(Invalid); };",
        "single.idl",
    );
    insta::assert_snapshot!("single_version_typescript_adapters", &files[0].1);
}

#[test]
fn changed_source_declarations_require_decisions_even_with_identical_wire_shapes() {
    let files = render(
        r#"
        @version enum V { v1, v2 };
        struct Item { @removed(V::v2) string value; @added(V::v2) string value; };
        interface Store { @post(path="/items") Item save(@body Item item); };
        "#,
        "replacement.idl",
    );
    assert!(files[0].1.contains("itemUpcastFromV1"));
    assert!(files[0].1.contains("ignored$value"));
    assert!(files[0].1.contains("itemDowncastToV1"));
    insta::assert_snapshot!("identical_declaration_typescript_adapters", &files[0].1);
}

#[test]
fn adapter_generation_rejects_unversioned_retired_and_incomplete_projected_inputs() {
    for (source, expected) in [
        ("struct Item { string id; };", "version_map"),
        (
            "@version enum V { v1,v2 }; interface Store { @removed(V::v2) @get(path=\"/retired\") string retired(); };",
            "removed operations",
        ),
        (
            "@version enum V { v1,v2 }; @removed(V::v2) interface Store { @get(path=\"/old\") string get(); }; @added(V::v2) interface Store { @get(path=\"/new\") string get(); };",
            "replaced operation",
        ),
        (
            "@version enum V { only }; interface Store { @server_stream @get(path=\"/events\") string events(); };",
            "stream and upgrade",
        ),
    ] {
        let result = xidlc::driver::Generator::new("typescript-rest-adapters".into())
            .generate_from_idl(source, Path::new("invalid.idl"), HashMap::new());
        let error = result
            .err()
            .expect("invalid adapter contract must fail")
            .to_string();
        assert!(error.contains(expected), "{expected}: {error}");
    }
    let source =
        "@version enum V { v1,v2 }; interface Store { @get(path=\"/items\") string get(); };";
    let result = xidlc::driver::Generator::new("typescript-rest-adapters".into())
        .generate_from_idl(
            source,
            Path::new("selected.idl"),
            HashMap::from([("api_version".into(), serde_json::json!("v1"))]),
        );
    assert!(
        result.is_err(),
        "incomplete HIR cannot consume a complete version map"
    );
}

#[test]
fn generated_symbols_reject_normalized_versions_scopes_and_cross_category_collisions() {
    for source in [
        r#"@version enum V { v1, V1 };
        interface Store { @get(path="/save") string save(); };"#,
        r#"@version enum V { only };
        module A_B { interface Store { @get(path="/one") string save(); }; };
        module A { module B { interface Store { @get(path="/two") long save(); }; }; };"#,
        r#"@version enum V { only };
        module A { struct B { string id; }; };
        struct A_B { long id; };
        interface Store { @get(path="/one") A::B first(); @get(path="/two") A_B second(); };"#,
        r#"@version enum V { only };
        struct StoreSaveResultToOnly { string id; };
        interface Store { @get(path="/save") StoreSaveResultToOnly save(); };"#,
    ] {
        let result = xidlc::driver::Generator::new("typescript-rest-adapters".into())
            .generate_from_idl(source, Path::new("collision.idl"), HashMap::new());
        let error = result
            .err()
            .expect("ambiguous symbols must fail")
            .to_string();
        assert!(error.contains("generated symbol"), "{error}");
        assert!(error.contains("ambiguous"), "{error}");
        assert!(error.contains("collision"), "{error}");
    }
}

#[test]
fn ordinary_targets_match_the_master_artifacts_byte_for_byte() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut targets = vec!["typescript-rest"];
    if cfg!(feature = "gen-rust-axum") {
        targets.push("rust-axum");
    }
    for target in targets {
        for fixture in [
            "versioned_adapters",
            "versioned_adapters_business",
            "versioned_rest",
            "wire_contract",
            "http_body_shape",
        ] {
            let baseline = root.join(format!("tests/adapter_baseline/{target}.{fixture}.json"));
            let expected: std::collections::BTreeMap<String, String> = serde_json::from_str(
                &std::fs::read_to_string(&baseline).expect("master baseline snapshot"),
            )
            .expect("baseline artifact manifest");
            let source = root.join(format!("../bdd/features/data/{fixture}.idl"));
            let generated = xidlc::driver::Generator::new(target.into())
                .generate_from_idl(
                    &std::fs::read_to_string(&source).expect("contract"),
                    &source,
                    HashMap::from([
                        ("enable_client".into(), serde_json::json!(true)),
                        ("enable_server".into(), serde_json::json!(true)),
                    ]),
                )
                .expect("ordinary generation")
                .into_iter()
                .map(|file| (file.path().to_string(), file.content().to_string()))
                .collect::<std::collections::BTreeMap<_, _>>();
            assert_eq!(
                generated, expected,
                "{target} artifacts for {fixture} differ from master d98bc37a"
            );
        }
    }
}
