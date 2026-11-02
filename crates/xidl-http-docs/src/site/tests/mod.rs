use super::Site;
use crate::config::DocumentationConfig;
use std::collections::BTreeMap;
use xidlc::generate::hir_gen::HirGen;
use xidlc::jsonrpc::{
    Artifact, COMPILATION_FORMAT, Codegen, CodegenInput, CompilationInput, CompilationUnit,
    VersionInput,
};

mod package;
mod playgrounds;

fn compilation() -> CompilationInput {
    compilation_for(include_str!("fixtures/catalog.idl"))
}

fn compilation_for(source: &str) -> CompilationInput {
    let props = serde_json::from_value(serde_json::json!({
        "idl": source,
        "target_lang": "http-docs", "hir_kind": "http", "version_layout": "per_version"
    }))
    .unwrap();
    let artifacts = HirGen
        .generate(
            CodegenInput::RpcHir(xidl_parser::hir::Specification(vec![])),
            "catalog.idl".into(),
            props,
        )
        .unwrap();
    let mut version_map = None;
    let inputs = artifacts
        .into_iter()
        .map(|artifact| {
            let Artifact::RestHir(data) = artifact else {
                panic!("catalog fixture requires HTTP inputs");
            };
            version_map = data
                .props
                .get("version_map")
                .map(|value| serde_json::from_value(value.clone()).unwrap());
            VersionInput {
                version: data
                    .props
                    .get("api_version")
                    .and_then(|v| v.as_str())
                    .map(str::to_owned),
                input: CodegenInput::RestHir(data.rest_hir),
            }
        })
        .collect();
    CompilationInput {
        format: COMPILATION_FORMAT,
        units: vec![CompilationUnit {
            id: "catalog".into(),
            is_root: true,
            imports: vec![],
            version_map,
            inputs,
        }],
    }
}

fn publication() -> BTreeMap<String, String> {
    Site::build(
        compilation(),
        &DocumentationConfig {
            base_url: "https://docs.example.test/reference/".into(),
            ..DocumentationConfig::default()
        },
    )
    .unwrap()
    .into_iter()
    .map(|artifact| {
        let Artifact::File(file) = artifact else {
            panic!("documentation must emit files")
        };
        (file.path, file.content)
    })
    .collect()
}

#[test]
fn versioned_publication_snapshots_preserve_identity_facts_and_framework_input() {
    let files = publication();
    insta::assert_snapshot!(
        "catalog_changes",
        files["public/changelog/catalog/v1-to-v2.md"]
    );
    insta::assert_snapshot!(
        "catalog_reference",
        files["src/content/docs/api/catalog/v2/types/page-_Product.md"]
    );
    insta::assert_json_snapshot!(
        "catalog_navigation",
        serde_json::from_str::<serde_json::Value>(&files["xidl-site.json"]).unwrap()
    );
    assert!(files["public/api/catalog/v2/types/_Product.md"].contains("产品目录 💳"));
    assert!(
        files["public/api/catalog/v2/types/_Product.md"]
            .contains("Display names use **Markdown** and `UTF-8`.\n\n- Keep a stable product ID.")
    );
    assert!(!files["public/changelog/catalog/v1-to-v2.md"].contains("@doc"));
    assert!(files["public/llms-full.txt"].contains(r"# \_Product"));
    assert!(
        !files
            .keys()
            .any(|path| path.ends_with(".html") || path.ends_with("docs.js"))
    );
}

#[test]
fn semantic_envelope_rejects_unsupported_formats_and_duplicate_units() {
    let mut unsupported = compilation();
    unsupported.format += 1;
    assert!(
        Site::build(unsupported, &DocumentationConfig::default())
            .unwrap_err()
            .to_string()
            .contains("unsupported compilation format")
    );
    let mut duplicate = compilation();
    duplicate.units.push(duplicate.units[0].clone());
    assert!(
        Site::build(duplicate, &DocumentationConfig::default())
            .unwrap_err()
            .to_string()
            .contains("duplicate compilation unit 'catalog'")
    );
}

#[test]
fn excluded_referenced_type_fails_but_excluding_its_only_operation_publishes_an_empty_site() {
    let mut config = DocumentationConfig::default();
    config.exclude.insert("catalog::_Product".into());
    assert!(
        Site::build(compilation(), &config)
            .unwrap_err()
            .to_string()
            .contains("references excluded type")
    );
    config.exclude.insert("catalog::Catalog::read".into());
    let files = Site::build(compilation(), &config).unwrap();
    assert!(
        !files
            .iter()
            .any(|artifact| artifact.as_file().path.contains("types/"))
    );
}
