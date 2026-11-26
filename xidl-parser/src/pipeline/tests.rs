use super::{CompileOptions, Pipeline, ProjectedHir};
use crate::error::ParseError;
use crate::hir::FsIncludeResolver;
use std::collections::HashMap;

#[test]
fn compile_source_produces_projections_and_version_map_in_one_pass() {
    let source = r#"
        @version enum Versions { v1, v2 };
        struct Item {
            string id;
            @added(Versions::v2) @optional string note;
        };
        interface Items {
            @get(path = "/items/{id}") Item get_item(@path string id);
        };
    "#;
    let props = HashMap::from([(
        "hir_kind".to_string(),
        serde_json::Value::String("http".to_string()),
    )]);
    let options = CompileOptions::new("items.idl", props).with_version_map(true);
    let mut resolver = FsIncludeResolver;
    let compiled = Pipeline::compile_source(source, options, &mut resolver).unwrap();

    assert_eq!(compiled.projections.len(), 1);
    assert!(matches!(compiled.projections[0].hir, ProjectedHir::Http(_)));
    let version_map = compiled
        .version_map
        .expect("versioned contract should produce version_map");
    assert_eq!(version_map.versions.len(), 2);
    assert!(compiled.diagnostics.is_empty());
}

#[test]
fn compile_source_surfaces_syntax_diagnostics() {
    let broken = "struct Broken { string id };";
    let options = CompileOptions::new("broken.idl", HashMap::new());
    let mut resolver = FsIncludeResolver;
    let err = Pipeline::compile_source(broken, options, &mut resolver).unwrap_err();
    assert!(matches!(err, ParseError::Diagnostics(_)));
}

#[test]
fn compile_source_supports_jsonrpc_and_per_version_projection() {
    let source = r#"
        @version enum Versions { v1, v2 };
        interface Echo {
            string ping(string msg);
        };
    "#;
    let props = HashMap::from([
        (
            "hir_kind".to_string(),
            serde_json::Value::String("jsonrpc".to_string()),
        ),
        (
            "version_layout".to_string(),
            serde_json::Value::String("per_version".to_string()),
        ),
    ]);
    let options = CompileOptions::new("echo.idl", props);
    let mut resolver = FsIncludeResolver;
    let compiled = Pipeline::compile_source(source, options, &mut resolver).unwrap();

    assert_eq!(compiled.projections.len(), 2);
    assert!(
        compiled
            .projections
            .iter()
            .all(|projection| matches!(projection.hir, ProjectedHir::JsonRpc(_)))
    );
    assert!(compiled.version_map.is_none());
}

#[test]
fn build_version_map_handles_versioned_and_unversioned_contracts() {
    let mut resolver = FsIncludeResolver;
    let unversioned = "struct Item { string id; };";
    let options = CompileOptions::new("plain.idl", HashMap::new()).with_version_map(false);
    assert!(
        Pipeline::build_version_map(unversioned, options, &mut resolver)
            .unwrap()
            .is_none()
    );

    let versioned = r#"
        @version enum Versions { v1, v2 };
        struct Item { string id; };
    "#;
    let options = CompileOptions::new("ver.idl", HashMap::new()).with_version_map(false);
    let map = Pipeline::build_version_map(versioned, options, &mut resolver)
        .unwrap()
        .expect("versioned contract should produce map");
    assert_eq!(map.versions.len(), 2);
}

#[test]
fn compile_source_validates_version_selection_before_building_version_map() {
    let source = r#"
        @version enum Versions { v1, v2 };
        @removed(Versions::v2) struct Old { string id; };
        struct Pet { Old old; };
    "#;
    let props = HashMap::from([(
        "api_version".to_string(),
        serde_json::Value::String("9.9".to_string()),
    )]);
    let options = CompileOptions::new("dangling.idl", props).with_version_map(true);
    let mut resolver = FsIncludeResolver;
    let err = Pipeline::compile_source(source, options, &mut resolver).unwrap_err();
    assert!(
        err.to_string().contains("version 9.9 is not declared"),
        "{err}"
    );
}
