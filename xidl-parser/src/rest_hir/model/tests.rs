use super::*;

#[test]
fn from_props_and_find_interface_cover_success_and_failure_paths() {
    let err = RestHirDocument::from_props(&hir::ParserProperties::new()).expect_err("missing prop");
    assert!(
        matches!(err, ParseError::Message(message) if message.contains("missing rest_hir properties"))
    );

    let doc = RestHirDocument {
        spec: hir::Specification(Vec::new()),
        document: HttpDocumentMetadata::default(),
        interfaces: vec![HttpInterface {
            name: "CityApi".to_string(),
            module_path: vec!["api".to_string()],
            operations: Vec::new(),
        }],
    };
    let mut props = hir::ParserProperties::new();
    props.insert("rest_hir".to_string(), serde_json::to_value(&doc).unwrap());

    let parsed = RestHirDocument::from_props(&props).expect("rest_hir");
    assert!(
        parsed
            .find_interface(&["api".to_string()], "CityApi")
            .is_some()
    );
    assert!(
        parsed
            .find_interface(&["other".to_string()], "CityApi")
            .is_none()
    );
}
