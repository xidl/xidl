#![cfg(all(feature = "gen-openapi", feature = "gen-rust-axum"))]

use std::{collections::HashMap, path::Path};

const CATALOG: &str = include_str!("../../bdd/features/data/versioned_catalog.idl");

#[test]
fn generated_collection_contracts_allow_zero_occurrences() {
    let source = include_str!("../../bdd/features/data/http_collections.idl");
    let files = xidlc::driver::Generator::new("openapi".into())
        .generate_from_idl(source, Path::new("http_collections.idl"), HashMap::new())
        .expect("collection OpenAPI");
    let document: serde_json::Value = serde_json::from_str(files[0].content()).unwrap();
    let parameters = document["paths"]["/collections"]["get"]["parameters"]
        .as_array()
        .unwrap();
    assert_eq!(parameters.len(), 4);
    for parameter in parameters {
        assert_eq!(parameter["required"], false);
        assert_eq!(parameter["schema"]["type"], "array");
    }
    insta::assert_json_snapshot!("collection_openapi", document);

    let files = xidlc::driver::Generator::new("rust-axum".into())
        .generate_from_idl(source, Path::new("http_collections.idl"), HashMap::new())
        .expect("collection Rust requests");
    insta::assert_snapshot!("collection_rust", files[0].content());
}

#[test]
fn selected_catalog_openapi_freezes_both_media_representations() {
    let generate = |version: Option<&str>| {
        let props = version
            .map(|value| HashMap::from([("api_version".into(), value.into())]))
            .unwrap_or_default();
        xidlc::driver::Generator::new("openapi".into())
            .generate_from_idl(CATALOG, Path::new("versioned_catalog.idl"), props)
            .expect("catalog OpenAPI")
    };
    let complete = generate(None);
    assert_eq!(complete.len(), 2);
    let mut documents = std::collections::BTreeMap::new();
    for version in ["v1", "v2"] {
        let selected = generate(Some(version));
        assert_eq!(selected.len(), 1);
        let full = complete
            .iter()
            .find(|file| file.path() == selected[0].path())
            .unwrap();
        assert_eq!(full.content(), selected[0].content());
        let document: serde_json::Value = serde_json::from_str(full.content()).unwrap();
        let parameters = document["paths"]["/catalog"]["get"]["parameters"]
            .as_array()
            .unwrap();
        let media = parameters
            .iter()
            .find(|parameter| parameter["name"] == "media")
            .unwrap();
        assert_eq!(media["in"], "query");
        assert_eq!(media["required"], false);
        let tenant = parameters
            .iter()
            .find(|parameter| parameter["name"] == "tenant")
            .unwrap();
        assert_eq!(tenant["required"], true);
        documents.insert(version, document);
    }
    insta::assert_json_snapshot!("catalog_openapi_contracts", documents);
}
