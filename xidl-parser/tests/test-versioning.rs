use std::collections::HashMap;

use serde_json::{Value, json};
use xidl_parser::hir::ParserProperties;
use xidl_parser::rest_hir::ProjectedHir;
use xidl_parser::versioning::{API_VERSION_PROPERTY, LAYOUT_PER_VERSION, VERSION_LAYOUT_PROPERTY};

const PETSTORE: &str = r#"
#pragma xidlc package versioned_petstore

@version
enum ApiVersions {
    @rename("1.0") v1,
    @rename("2.0") v2,
};

struct Pet {
    string id;
    @added(ApiVersions::v2)
    @optional string note;
    @removed(ApiVersions::v2) string color;
};

interface Pets {
    @get(path = "/pets/{id}")
    Pet get(@path string id);

    @removed(ApiVersions::v2)
    @get(path = "/legacy")
    string legacy();
};
"#;

const CARRIERS: &str = r#"
#pragma xidlc package carriers

@version
enum Versions {
    @rename("1.0") v1,
    @rename("2.0") v2,
};

struct StructCarrier {
    @added(Versions::v2) int32 member;
};

union UnionCarrier switch (long) {
    case 0: @added(Versions::v2) int32 value;
};

enum EnumCarrier {
    @added(Versions::v2) Value,
};

@http(404)
exception ExceptionCarrier {
    @added(Versions::v2) string member;
};

@added(Versions::v2)
typedef int32 TypedefCarrier;

bitmask BitmaskCarrier {
    @added(Versions::v2) A
};
interface InterfaceCarrier {
    @added(Versions::v2)
    int32 operation(@added(Versions::v2) int32 parameter);
};
"#;

fn http_properties(extra: &[(&str, Value)]) -> ParserProperties {
    let mut properties = HashMap::from([("hir_kind".to_string(), json!("http"))]);
    for (key, value) in extra {
        properties.insert((*key).to_string(), value.clone());
    }
    properties
}

fn parse(source: &str) -> xidl_parser::typed_ast::Specification {
    xidl_parser::parser::parser_text(source).expect("IDL should parse")
}

#[test]
fn typed_ast_keeps_versioning_annotations() {
    let ast = parse(CARRIERS);
    insta::assert_debug_snapshot!("typed_ast_versioning_carriers", ast);
}

#[test]
fn hir_keeps_typed_versioning_annotations() {
    let typed = parse(CARRIERS);
    let mut properties = ParserProperties::new();
    properties.insert("expand_interface".to_string(), json!(false));
    let hir = xidl_parser::hir::Specification::from_typed_ast_with_properties_and_path(
        typed,
        properties,
        "carriers.idl",
    )
    .expect("HIR should lower");

    insta::assert_debug_snapshot!("hir_versioning_carriers", hir);
}

#[test]
fn merged_projection_wraps_one_module_per_version() {
    let typed = parse(PETSTORE);
    let projections = xidl_parser::hir::Specification::project_typed_ast_versions(
        typed,
        http_properties(&[]),
        "petstore.idl",
    )
    .expect("projection should succeed");

    assert_eq!(projections.len(), 1);
    assert!(projections[0].version.is_none());
    let ProjectedHir::Http(document) = &projections[0].hir else {
        panic!("expected an HTTP projection");
    };
    insta::assert_snapshot!(
        "hir_merged_versioning_petstore",
        serde_json::to_string_pretty(&document.spec).expect("serialize")
    );
}

#[test]
fn per_version_projection_follows_the_selected_version() {
    let typed = parse(PETSTORE);
    let projections = xidl_parser::hir::Specification::project_typed_ast_versions(
        typed,
        http_properties(&[(VERSION_LAYOUT_PROPERTY, json!(LAYOUT_PER_VERSION))]),
        "petstore.idl",
    )
    .expect("projection should succeed");

    let versions = projections
        .iter()
        .map(|projection| {
            projection
                .version
                .as_ref()
                .expect("per-version projections carry a version")
                .ident
                .clone()
        })
        .collect::<Vec<_>>();
    assert_eq!(versions, vec!["v1", "v2"]);

    let ProjectedHir::Http(v2) = &projections[1].hir else {
        panic!("expected an HTTP projection");
    };
    insta::assert_snapshot!(
        "hir_per_version_v2_petstore",
        serde_json::to_string_pretty(&v2.spec).expect("serialize")
    );
}

#[test]
fn selection_narrows_the_projected_versions() {
    let typed = parse(PETSTORE);
    let projections = xidl_parser::hir::Specification::project_typed_ast_versions(
        typed,
        http_properties(&[
            (VERSION_LAYOUT_PROPERTY, json!(LAYOUT_PER_VERSION)),
            (API_VERSION_PROPERTY, json!("2.0")),
        ]),
        "petstore.idl",
    )
    .expect("projection should succeed");

    assert_eq!(projections.len(), 1);
    assert_eq!(
        projections[0]
            .version
            .as_ref()
            .expect("version")
            .ident
            .as_str(),
        "v2"
    );
}

#[test]
fn unknown_versions_and_unversioned_contracts_are_rejected() {
    let typed = parse(PETSTORE);
    let error = xidl_parser::hir::Specification::project_typed_ast_versions(
        typed,
        http_properties(&[(API_VERSION_PROPERTY, json!("9.9"))]),
        "petstore.idl",
    )
    .expect_err("unknown version should fail");
    assert!(
        error.to_string().contains("version 9.9 is not declared"),
        "{error}"
    );

    let typed = parse("struct Pet { string id; };");
    let error = xidl_parser::hir::Specification::project_typed_ast_versions(
        typed,
        http_properties(&[(API_VERSION_PROPERTY, json!("v1"))]),
        "petstore.idl",
    )
    .expect_err("selection without a version enum should fail");
    assert!(
        error
            .to_string()
            .contains("requires a @version enum in the specification"),
        "{error}"
    );
}

#[test]
fn unversioned_contracts_keep_their_single_projection() {
    let typed = parse("#pragma xidlc package plain\nstruct Pet { string id; };");
    let projections = xidl_parser::hir::Specification::project_typed_ast_versions(
        typed,
        http_properties(&[]),
        "plain.idl",
    )
    .expect("projection should succeed");

    assert_eq!(projections.len(), 1);
    assert!(projections[0].version.is_none());
    let ProjectedHir::Http(document) = &projections[0].hir else {
        panic!("expected an HTTP projection");
    };
    assert!(
        document
            .spec
            .0
            .iter()
            .all(|definition| !matches!(definition, xidl_parser::hir::Definition::ModuleDcl(_)))
    );
}

#[test]
fn selector_param_collision_fails_http_projection() {
    let source = r#"
#pragma xidlc package collision

@version
@version_selector(header = "Api-Version")
enum ApiVersions {
    @rename("1.0") v1,
    @rename("2.0") v2,
};

interface Pets {
    @get(path = "/pets/{id}")
    Pet get(
        @path string id,
        @header @rename("Api-Version") string client_name
    );
};
"#;
    let typed = parse(source);
    let error = xidl_parser::hir::Specification::project_typed_ast_versions(
        typed,
        http_properties(&[(VERSION_LAYOUT_PROPERTY, json!(LAYOUT_PER_VERSION))]),
        "collision.idl",
    )
    .expect_err("a parameter bound under the selector name should fail projection");
    let message = error.to_string();
    assert!(
        message.contains("under the name of the @version_selector"),
        "{message}"
    );
    assert!(
        message.contains("header parameter 'Api-Version'"),
        "{message}"
    );
    assert!(
        message.contains("'Pets::get'") || message.contains("operation 'get'"),
        "{message}"
    );
}
