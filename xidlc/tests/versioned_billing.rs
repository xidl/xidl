#![cfg(all(
    feature = "gen-openapi",
    feature = "gen-rust-axum-adapters",
    feature = "gen-typescript-rest-adapters"
))]

use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
};

const BILLING: &str = include_str!("../../bdd/features/data/versioned_billing.idl");

fn generate(target: &str, version: Option<&str>) -> Vec<xidlc::driver::File> {
    let props = version
        .map(|version| HashMap::from([("api_version".into(), version.into())]))
        .unwrap_or_default();
    xidlc::driver::Generator::new(target.into())
        .generate_from_idl(BILLING, Path::new("versioned_billing.idl"), props)
        .expect("billing contract generation")
}

#[test]
fn selected_billing_openapi_preserves_every_frozen_contract() {
    let complete = generate("openapi", None);
    assert_eq!(complete.len(), 2);
    let documents: BTreeMap<_, serde_json::Value> = complete
        .iter()
        .map(|file| {
            (
                file.path().to_owned(),
                serde_json::from_str(file.content()).expect("generated OpenAPI JSON"),
            )
        })
        .collect();
    for version in ["v1", "v2"] {
        let selected = generate("openapi", Some(version));
        assert_eq!(selected.len(), 1);
        let full = complete
            .iter()
            .find(|file| file.path() == selected[0].path())
            .expect("selected document in complete history");
        assert_eq!(selected[0].content(), full.content());
    }
    insta::assert_json_snapshot!("billing_openapi_contracts", documents);
}

#[test]
fn billing_adapters_leave_http_version_selection_to_the_application() {
    for target in ["rust-axum-adapters", "typescript-rest-adapters"] {
        let files = generate(target, None);
        assert_eq!(files.len(), 1);
        let source = files[0].content();
        for policy in [
            "HeaderMap",
            "headers.get(",
            "req.headers",
            "resolve_version(",
            "X-Billing-Version",
        ] {
            assert!(
                !source.contains(policy),
                "{target} must leave {policy} to the application"
            );
        }
        assert!(source.contains("2025-01-01"));
        assert!(source.contains("2025-04-01"));
    }
}
