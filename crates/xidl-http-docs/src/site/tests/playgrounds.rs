use super::*;
use serde_json::Value;

#[test]
fn playgrounds_publish_reachable_models_and_selected_wire_versions() {
    let files = publication();
    for (version, wire, present, absent) in [
        ("v1", "2026-01", "label", "name"),
        ("v2", "2026-06", "name", "label"),
    ] {
        let document: Value =
            serde_json::from_str(&files[&format!("public/openapi/catalog/{version}.json")])
                .unwrap();
        let operation = &document["paths"]["/products/{id}"]["get"];
        assert_eq!(operation["description"], "Read a product by ID.");
        let parameter = operation["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .find(|parameter| parameter["name"] == "X-Catalog-Version")
            .unwrap();
        assert_eq!(parameter["schema"]["enum"], serde_json::json!([wire]));
        assert_eq!(parameter["schema"]["default"], wire);
        assert_eq!(parameter["required"], false);
        let schemas = document["components"]["schemas"].as_object().unwrap();
        assert_eq!(
            schemas.len(),
            2,
            "only the product and HTTP error are reachable"
        );
        let schema = schemas
            .values()
            .find(|schema| schema["properties"].get("id").is_some())
            .unwrap();
        assert_eq!(
            schema["description"],
            concat!(
                "产品目录 💳\n\n",
                "Display names use **Markdown** and `UTF-8`.\n\n",
                "- Keep a stable product ID.\n",
                "- Read the name in the selected API version."
            )
        );
        assert!(schema["properties"].get(present).is_some());
        assert!(schema["properties"].get(absent).is_none());
        if version == "v2" {
            assert_eq!(
                schema["properties"]["name"]["description"],
                "Public display name."
            );
        }
        for reference in crate::links::Links::references(&document) {
            assert!(reference.starts_with("#/components/schemas/"));
            assert!(
                document
                    .pointer(reference.strip_prefix('#').unwrap())
                    .is_some(),
                "{reference}"
            );
        }
        assert!(
            files[&format!("src/content/docs/api/catalog/{version}/types/page-_Product.md")]
                .contains(&format!("/reference/playground/catalog/{version}/"))
        );
    }
}

#[test]
fn removing_all_operations_removes_playgrounds_and_downloads() {
    let config = DocumentationConfig {
        exclude: ["catalog::Catalog::read".into()].into(),
        ..DocumentationConfig::default()
    };
    let files = Site::build(compilation(), &config).unwrap();
    assert!(
        !files
            .iter()
            .any(|artifact| artifact.as_file().path.starts_with("public/openapi/"))
    );
    let navigation = files
        .iter()
        .find(|artifact| artifact.as_file().path == "xidl-site.json")
        .unwrap();
    let navigation: Value = serde_json::from_str(&navigation.as_file().content).unwrap();
    assert_eq!(navigation["playgrounds"], serde_json::json!([]));
}

#[test]
fn published_closure_keeps_keyword_named_fields_and_excludes_private_interface_tags() {
    let source = r#"
        struct Detail { string value; };
        struct Item { Detail description; Detail examples; Detail default; };
        interface Public { @get(path = "/items") Item read(); };
        interface Internal { @get(path = "/private") Detail read(); };
    "#;
    let config = DocumentationConfig {
        exclude: ["catalog::Internal".into()].into(),
        ..DocumentationConfig::default()
    };
    let files = Site::build(compilation_for(source), &config).unwrap();
    let text = &files
        .iter()
        .find(|artifact| artifact.as_file().path == "public/openapi/catalog/unversioned.json")
        .unwrap()
        .as_file()
        .content;
    let document: Value = serde_json::from_str(text).unwrap();
    assert!(document["paths"].get("/private").is_none());
    assert!(
        !document["tags"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tag| tag["name"] == "Internal")
    );
    let schemas = document["components"]["schemas"].as_object().unwrap();
    let item = schemas
        .values()
        .find(|schema| schema["title"] == "Item")
        .unwrap();
    for name in ["description", "examples", "default"] {
        let reference = item["properties"][name]["$ref"].as_str().unwrap();
        assert_eq!(
            document
                .pointer(reference.strip_prefix('#').unwrap())
                .unwrap()["title"],
            "Detail"
        );
    }
    // JSON payload references are user data, while identically named fields are schemas.
    let payload = serde_json::json!({
        "properties": {"examples": {"$ref": "#/components/schemas/Detail"}},
        "example": {"$ref": "literal-payload", "description": "<b>literal</b>"},
        "default": {"$ref": "literal-default"}
    });
    assert_eq!(
        crate::links::Links::references(&payload),
        ["#/components/schemas/Detail".into()].into()
    );
}

#[test]
fn a_newly_reachable_type_with_identical_source_has_an_explicit_empty_diff() {
    let source = include_str!("fixtures/catalog.idl").replace(
        "    string id;",
        "    string id;\n    @added(ApiVersion::v2) AlwaysAvailable extra;\n    @added(ApiVersion::v2) AddedRecord fresh;",
    ) + "\nstruct AlwaysAvailable { string value; };\n@added(ApiVersion::v2) struct AddedRecord { string value; };";
    let files = Site::build(compilation_for(&source), &DocumentationConfig::default()).unwrap();
    let changes = &files
        .iter()
        .find(|artifact| artifact.as_file().path == "public/changelog/catalog/v1-to-v2.md")
        .unwrap()
        .as_file()
        .content;
    assert!(changes.contains("No IDL changes for this declaration."));
    assert!(changes.contains("No declaration published in this version."));
    assert!(!changes.contains("```json") && !changes.contains("\nnull\n"));
    assert!(changes.contains("```idl"));
    assert!(changes.contains("Compare IDL · AlwaysAvailable"));
}
