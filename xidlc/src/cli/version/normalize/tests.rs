//! Normalization coverage: keyword stripping, name-map protection,
//! component pruning and canonical ordering.

use super::normalize_document;
use serde_json::{Value, json};

fn normalized(value: Value) -> Value {
    let mut document = value;
    normalize_document(&mut document);
    document
}

#[test]
fn documentation_keywords_disappear_at_keyword_positions() {
    let out = normalized(json!({
        "openapi": "3.1.0",
        "info": {"title": "petstore", "version": "1.0", "description": "prose"},
        "servers": [{"url": "http://localhost"}],
        "tags": [{"name": "Pets", "description": "prose"}],
        "paths": {"/pets": {"get": {
            "summary": "list",
            "deprecated": true,
            "parameters": [],
            "responses": {"200": {"description": "ok"}}
        }}}
    }));

    assert!(out.get("openapi").is_none());
    assert!(out.get("servers").is_none());
    assert!(out["info"].get("description").is_none());
    assert!(out["paths"]["/pets"]["get"].get("summary").is_none());
    assert!(out["paths"]["/pets"]["get"].get("deprecated").is_none());
    assert!(
        out["paths"]["/pets"]["get"]["responses"]["200"]
            .get("description")
            .is_none()
    );
    assert_eq!(out["info"]["version"], "1.0");
    assert_eq!(out["info"]["title"], "petstore");
}

#[test]
fn name_keyed_maps_keep_their_member_names() {
    let out = normalized(json!({
        "paths": {"/described": {"get": {"responses": {"200": {
            "content": {"application/json": {
                "schema": {"$ref": "#/components/schemas/description"}
            }},
            "headers": {"description": {"schema": {"type": "string"}}}
        }}}}},
        "components": {"schemas": {
            "description": {"type": "object", "properties": {
                "description": {"type": "string", "description": "prose"}
            }}
        }}
    }));

    let schema = &out["components"]["schemas"]["description"];
    assert_eq!(schema["type"], "object");
    assert_eq!(schema["properties"]["description"]["type"], "string");
    assert!(
        schema["properties"]["description"]
            .get("description")
            .is_none()
    );
    let header = &out["paths"]["/described"]["get"]["responses"]["200"]["headers"]["description"];
    assert_eq!(header["schema"]["type"], "string");
}

#[test]
fn unreferenced_components_are_pruned_transitively() {
    let out = normalized(json!({
        "paths": {"/x": {"get": {"responses": {"200": {"content": {
            "application/json": {"schema": {"$ref": "#/components/schemas/Kept"}}}}}}}},
        "components": {"schemas": {
            "Kept": {"type": "object", "properties": {
                "child": {"$ref": "#/components/schemas/Child"}
            }},
            "Child": {"type": "string"},
            "Orphan": {"type": "string"},
            "GrandOrphan": {"type": "string"}
        }}
    }));

    let schemas = &out["components"]["schemas"];
    assert!(schemas.get("Kept").is_some());
    assert!(schemas.get("Child").is_some());
    assert!(schemas.get("Orphan").is_none());
    assert!(schemas.get("GrandOrphan").is_none());
}

#[test]
fn set_like_arrays_are_sorted_and_parameters_keyed() {
    let out = normalized(json!({
        "tags": ["zoo", "alpha"],
        "paths": {"/x": {"get": {"parameters": [
            {"name": "b", "in": "query"},
            {"name": "a", "in": "query"},
            {"name": "h", "in": "header"}
        ]}}}
    }));

    assert_eq!(out["tags"], json!(["alpha", "zoo"]));
    let parameters = out["paths"]["/x"]["get"]["parameters"]
        .as_array()
        .expect("array");
    let order: Vec<_> = parameters
        .iter()
        .map(|parameter| {
            (
                parameter["in"].as_str().unwrap(),
                parameter["name"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(order, [("header", "h"), ("query", "a"), ("query", "b")]);
}
