use super::RustRenderer;
use super::generate_with_properties;
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;
use xidl_parser::hir;

#[test]
fn preserves_numeric_file_stem_in_output_path() {
    let artifacts = generate_with_properties(
        &hir::Specification(vec![]),
        Path::new("e2e_test.idl"),
        &HashMap::new(),
    )
    .expect("rust generation should succeed");

    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].try_as_file().unwrap().path, "e2e_test.rs");
}

#[test]
fn rejects_interface_declarations() {
    let spec = hir::Specification(vec![hir::Definition::InterfaceDcl(hir::InterfaceDcl {
        annotations: Vec::new(),
        decl: hir::InterfaceDclInner::InterfaceForwardDcl(hir::InterfaceForwardDcl {
            ident: "Greeter".to_string(),
        }),
    })]);

    let error = generate_with_properties(&spec, Path::new("api.idl"), &HashMap::new())
        .expect_err("the rust target no longer supports interfaces");
    assert!(
        error
            .to_string()
            .contains("does not support the `interface Greeter`"),
        "unexpected error: {error}"
    );
}

#[test]
fn struct_template_uses_serde_rename_semantics() {
    let renderer = RustRenderer::new(HashMap::new()).expect("renderer");
    let rendered = renderer
        .render_template(
            "struct.rs.j2",
            &json!({
                "doc": [],
                "rust_attrs": [],
                "derive": ["::serde::Serialize", "::serde::Deserialize"],
                "enable_serde_attrs": true,
                "ident": "RenameStruct",
                "parent": null,
                "members": [{
                    "doc": [],
                    "rust_attrs": [],
                    "serde_rename": "wireName",
                    "serde_skip": false,
                    "name": "plain_name",
                    "ty": "String"
                }]
            }),
        )
        .expect("render struct template");

    assert!(rendered.contains("#[serde(rename = \"wireName\")]"));
    assert!(
        !rendered.contains("rename_all"),
        "the struct template never re-derives names at runtime"
    );
}

#[test]
fn enum_template_uses_serde_rename_semantics() {
    let renderer = RustRenderer::new(HashMap::new()).expect("renderer");
    let rendered = renderer
        .render_template(
            "enum.rs.j2",
            &json!({
                "doc": [],
                "rust_attrs": [],
                "ident": "RenameEnum",
                "has_serde_serialize": true,
                "has_serde_deserialize": true,
                "members": [{
                    "doc": [],
                    "rust_attrs": [],
                    "serde_rename": "wire-value",
                    "is_default": true,
                    "name": "LocalValue"
                }]
            }),
        )
        .expect("render enum template");

    assert!(rendered.contains("#[serde(rename = \"wire-value\")]"));
    assert!(
        !rendered.contains("rename_all"),
        "the enum template never re-derives names at runtime"
    );
}
