use std::collections::HashMap;
use std::path::Path;

use xidlc::generate::hir_gen::HirGen;
use xidlc::jsonrpc::{Artifact, Codegen, CodegenInput};

const VERSIONED_REST: &str = include_str!("../../bdd/features/data/versioned_rest.idl");

fn version_map_files(target: &str) -> Vec<(String, String)> {
    let mut generator = xidlc::driver::Generator::new(target.to_string());
    generator
        .generate_from_idl(VERSIONED_REST, Path::new("api.idl"), HashMap::new())
        .expect("version map generation")
        .into_iter()
        .map(|file| (file.path().to_string(), file.content().to_string()))
        .collect()
}

#[test]
fn version_map_target_is_a_stable_json_artifact() {
    let first = version_map_files("version-map");
    let second = version_map_files("version-map");

    assert_eq!(first, second, "version-map output must be byte-stable");
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].0, "api.version-map.json");
    assert!(first[0].1.ends_with('\n'));
    let document: serde_json::Value = serde_json::from_str(&first[0].1).expect("valid JSON");
    assert_eq!(document["format"], 2);
    assert_eq!(document["versions"][0]["id"], "v1");
    insta::assert_snapshot!("version_map_artifact", &first[0].1);
}

#[test]
fn version_map_accepts_the_underscore_alias_without_reentering_generation() {
    let hyphen = version_map_files("version-map");
    let underscore = version_map_files("version_map");
    assert_eq!(hyphen, underscore);
}

#[test]
fn version_map_target_rejects_unversioned_contracts() {
    let mut generator = xidlc::driver::Generator::new("version-map".to_string());
    let error = match generator.generate_from_idl(
        "struct Item { string id; };",
        Path::new("api.idl"),
        HashMap::new(),
    ) {
        Ok(_) => panic!("unversioned contract must fail"),
        Err(error) => error,
    };
    assert!(
        error.to_string().contains("requires a @version enum"),
        "{error}"
    );
}

#[test]
fn hir_artifact_carries_the_same_version_map() {
    let props = HashMap::from([
        ("idl".to_string(), serde_json::json!(VERSIONED_REST)),
        ("target_lang".to_string(), serde_json::json!("openapi")),
        ("hir_kind".to_string(), serde_json::json!("http")),
    ]);
    let artifacts = HirGen
        .generate(
            CodegenInput::new_rpc_hir(xidl_parser::hir::Specification(vec![])),
            "api.idl".into(),
            props,
        )
        .expect("HIR generation");
    let Artifact::RestHir(artifact) = &artifacts[0] else {
        panic!("expected a REST HIR artifact");
    };
    let version_map = artifact.props.get("version_map").expect("version map prop");
    assert_eq!(version_map["format"], 2);
    assert_eq!(version_map["contract"], "api.idl");
    let file_map: serde_json::Value = serde_json::from_str(&version_map_files("version-map")[0].1)
        .expect("version-map artifact JSON");
    assert_eq!(version_map, &file_map);
}

#[test]
fn jsonrpc_generation_does_not_invent_http_operations_in_the_map() {
    let source = r#"
        @version
        enum Versions { v1, v2 };
        interface Items {
            string read(
                @removed(Versions::v2) string legacy,
                @added(Versions::v2) string current
            );
        };
    "#;
    let props = HashMap::from([
        ("idl".to_string(), serde_json::json!(source)),
        ("target_lang".to_string(), serde_json::json!("openrpc")),
        ("hir_kind".to_string(), serde_json::json!("jsonrpc")),
    ]);
    let artifacts = HirGen
        .generate(
            CodegenInput::new_rpc_hir(xidl_parser::hir::Specification(vec![])),
            "rpc.idl".into(),
            props,
        )
        .expect("JSON-RPC HIR generation");
    let version_map = match &artifacts[0] {
        Artifact::JsonRpcHir(artifact) => artifact.props.get("version_map"),
        _ => panic!("expected a JSON-RPC HIR artifact"),
    }
    .expect("version map prop");
    assert!(version_map["steps"][0].get("operations").is_none());
}

#[test]
fn unit_graph_integer_facts_reach_the_hir_version_map() {
    let root = tempfile::tempdir().expect("version map import graph");
    std::fs::write(
        root.path().join("dep.idl"),
        "const unsigned long TWO = 2; const unsigned long THREE = 3;",
    )
    .unwrap();
    let input = root.path().join("api.idl");
    std::fs::write(
        &input,
        r#"
        import "dep.idl";
        @version enum V { v1, v2 };
        @removed(V::v2) module cfg {
            const unsigned long WIDTH = dep::TWO;
            typedef long Slots[WIDTH];
        };
        @added(V::v2) module cfg {
            const unsigned long WIDTH = dep::THREE;
            typedef long Slots[WIDTH];
        };
    "#,
    )
    .unwrap();
    let graph = xidlc::units::UnitGraph::build(&[input], &Default::default()).unwrap();
    let unit = graph.units().last().expect("importing unit");
    let properties = HashMap::from([
        ("idl".into(), serde_json::json!(unit.source())),
        ("target_lang".into(), "rust".into()),
        (
            xidl_parser::semantic::INTEGER_IMPORTS_PROPERTY.into(),
            serde_json::to_value(graph.integer_imports(unit)).unwrap(),
        ),
    ]);
    let artifacts = HirGen
        .generate(
            CodegenInput::new_rpc_hir(xidl_parser::hir::Specification(vec![])),
            unit.generation_input().to_string_lossy().into_owned(),
            properties,
        )
        .expect("HIR with imported array dimensions");
    let Artifact::Hir(artifact) = &artifacts[0] else {
        panic!("expected a source HIR artifact");
    };
    let change = &artifact.props["version_map"]["steps"][0]["types"]["cfg::Slots"];
    assert_eq!(change["from"]["declarators"][0]["lengths"][0], "2");
    assert_eq!(change["to"]["declarators"][0]["lengths"][0], "3");
}
