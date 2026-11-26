use super::{
    CollectionBoundsPass, CompileOptions, DiagnoseOptions, HirProjectionKind, HttpUnionPass,
    JsonRpcProtocolPass, Pipeline, ProjectedHir, RestProtocolPass, StructInheritancePass,
    UnionCasePass, VersioningPass,
};
use crate::diagnostic::{DiagnosticBag, DiagnosticCode, HirDiagnosticPass};
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

#[test]
fn diagnose_source_collects_multiple_semantic_errors_in_one_pass() {
    let source = r#"
        enum Kind { alpha, beta };
        union Choice switch (Kind) {
            case alpha: string a;
            case beta: string b;
        };
        struct Base {
            string id;
            string name;
        };
        struct Child : Base {
            string id;
            string name;
        };
        struct Missing : NoSuchBase {
            sequence<string, 0> bad_seq;
            map<string, string, 0> bad_map;
            long bad_arr[0];
        };
    "#;
    let mut resolver = FsIncludeResolver;
    let bag = Pipeline::diagnose_source(
        source,
        DiagnoseOptions::new("semantic.idl").with_protocol(HirProjectionKind::Rpc),
        &mut resolver,
    );
    let messages: Vec<&str> = bag
        .as_slice()
        .iter()
        .map(|item| item.message.as_str())
        .collect();
    assert!(
        bag.as_slice()
            .iter()
            .all(|item| item.code == DiagnosticCode::SemanticError),
        "{messages:?}"
    );
    assert!(messages.iter().any(|m| m.contains("case label 'alpha'")));
    assert!(messages.iter().any(|m| m.contains("case label 'beta'")));
    assert!(messages.iter().any(|m| m.contains("field 'id' duplicates")));
    assert!(
        messages
            .iter()
            .any(|m| m.contains("field 'name' duplicates"))
    );
    assert!(messages.iter().any(|m| m.contains("inherits 'NoSuchBase'")));
    assert!(
        messages
            .iter()
            .any(|m| m.contains("sequence bound must be positive"))
    );
    assert!(
        messages
            .iter()
            .any(|m| m.contains("map bound must be positive"))
    );
    assert!(
        messages
            .iter()
            .any(|m| m.contains("array dimension for 'Missing::bad_arr' must be positive"))
    );
    assert!(messages.len() >= 8, "expected >=8 errors, got {messages:?}");
}

#[test]
fn diagnose_source_collects_multiple_versioning_errors_in_one_pass() {
    let source = r#"
        @version enum Versions { v1, v2 };
        struct Item {
            @added(Versions::missing_a) string first;
            @removed(Versions::missing_b) string second;
            @rename("dup") string a;
            @rename("dup") string b;
        };
        struct Other {
            @rename("same") string x;
            @rename("same") string y;
        };
    "#;
    let mut resolver = FsIncludeResolver;
    let bag = Pipeline::diagnose_source(
        source,
        DiagnoseOptions::new("versioning.idl").with_protocol(HirProjectionKind::Rpc),
        &mut resolver,
    );
    let messages: Vec<&str> = bag
        .as_slice()
        .iter()
        .map(|item| item.message.as_str())
        .collect();
    assert!(
        bag.as_slice()
            .iter()
            .all(|item| item.code == DiagnosticCode::VersioningError),
        "{messages:?}"
    );
    assert!(messages.iter().any(|m| m.contains("Versions::missing_a")));
    assert!(messages.iter().any(|m| m.contains("Versions::missing_b")));
    assert!(
        messages
            .iter()
            .any(|m| m.contains("Item") && m.contains("dup"))
    );
    assert!(
        messages
            .iter()
            .any(|m| m.contains("Other") && m.contains("same"))
    );
    assert!(messages.len() >= 4, "expected >=4 errors, got {messages:?}");
}

#[test]
fn diagnose_source_collects_multiple_http_protocol_errors_in_one_pass() {
    let source = r#"
        @http union Payload switch (::ContentType) {
            case ::ContentType::APPLICATION_JSON: string a[4];
            default: string b;
        };
        interface Api {
            @get(path = "/items/{id}") string missing_path();
            @get(path = "/dup") string first_dup();
            @get(path = "/dup") string second_dup();
            @get(path = "/bad-upgrade") @upgrade(protocol = "websocket") string bad_upgrade();
        };
    "#;
    let mut resolver = FsIncludeResolver;
    let bag = Pipeline::diagnose_source(source, DiagnoseOptions::new("http.idl"), &mut resolver);
    let messages: Vec<&str> = bag
        .as_slice()
        .iter()
        .map(|item| item.message.as_str())
        .collect();
    assert!(
        bag.as_slice()
            .iter()
            .all(|item| item.code == DiagnosticCode::ProtocolError),
        "{messages:?}"
    );
    assert!(
        messages
            .iter()
            .any(|m| m.contains("array case declarators are unsupported"))
    );
    assert!(messages.iter().any(|m| m.contains("no default")));
    assert!(
        messages
            .iter()
            .any(|m| m.contains("route template variable 'id' has no matching"))
    );
    assert!(messages.iter().any(|m| m.contains("duplicate HTTP route")));
    assert!(
        messages
            .iter()
            .any(|m| m.contains("@upgrade method 'bad_upgrade' must return void"))
    );
    assert!(messages.len() >= 5, "expected >=5 errors, got {messages:?}");
}

#[test]
fn diagnose_source_collects_multiple_jsonrpc_protocol_errors_in_one_pass() {
    let source = r#"
        interface Rpc {
            @server_stream @client_stream string conflicting_a();
            @server_stream @bidi_stream string conflicting_b();
            attribute string status;
            string get_attribute_status();
            readonly attribute string health;
            string get_attribute_health();
        };
    "#;
    let mut resolver = FsIncludeResolver;
    let bag = Pipeline::diagnose_source(
        source,
        DiagnoseOptions::new("rpc.idl").with_protocol(HirProjectionKind::JsonRpc),
        &mut resolver,
    );
    let messages: Vec<&str> = bag
        .as_slice()
        .iter()
        .map(|item| item.message.as_str())
        .collect();
    assert!(
        bag.as_slice()
            .iter()
            .all(|item| item.code == DiagnosticCode::ProtocolError),
        "{messages:?}"
    );
    assert_eq!(
        messages
            .iter()
            .filter(|m| m.contains("mutually exclusive"))
            .count(),
        2
    );
    assert!(messages.iter().any(|m| m.contains("attribute `status`")));
    assert!(messages.iter().any(|m| m.contains("attribute `health`")));
}

#[test]
fn diagnose_source_allows_disjoint_versioned_http_routes_and_combines_ast_and_hir_passes() {
    let valid_versioned = r#"
        @version enum Versions { v1, v2 };
        interface Pets {
            @removed(Versions::v2) @get(path = "/pets") string list_v1();
            @added(Versions::v2) @get(path = "/pets") string list_v2();
        };
    "#;
    let mut resolver = FsIncludeResolver;
    let valid_bag = Pipeline::diagnose_source(
        valid_versioned,
        DiagnoseOptions::new("pets.idl"),
        &mut resolver,
    );
    assert!(valid_bag.is_empty(), "{:?}", valid_bag.as_slice());

    let combined = r#"
        struct Base { string id; };
        struct Child : Base {
            @optinal string extra;
            string id;
        };
    "#;
    let bag = Pipeline::diagnose_source(
        combined,
        DiagnoseOptions::new("combined.idl"),
        &mut resolver,
    );
    assert!(
        bag.as_slice()
            .iter()
            .any(|item| item.code == DiagnosticCode::MisspelledAnnotation)
    );
    assert!(
        bag.as_slice()
            .iter()
            .any(|item| item.code == DiagnosticCode::SemanticError)
    );
}

#[test]
fn individual_hir_diagnostic_passes_implement_trait() {
    let mut resolver = FsIncludeResolver;
    let ast = crate::parser::parser_text("struct Item { string id; };").unwrap();
    let spec =
        Pipeline::lower_source_spec(ast, std::path::Path::new("item.idl"), &mut resolver).unwrap();
    let mut bag = DiagnosticBag::new();
    let passes: Vec<Box<dyn HirDiagnosticPass>> = vec![
        Box::new(UnionCasePass),
        Box::new(HttpUnionPass),
        Box::new(StructInheritancePass),
        Box::new(CollectionBoundsPass::default()),
        Box::new(VersioningPass),
        Box::new(RestProtocolPass::default()),
        Box::new(JsonRpcProtocolPass),
    ];
    for pass in passes {
        pass.run(&spec, &mut bag);
    }
    assert!(bag.is_empty());
}
