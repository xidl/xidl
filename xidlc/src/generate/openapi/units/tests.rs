use super::DocumentUnits;
use xidl_parser::hir::ScopedName;

#[test]
fn imported_document_versions_come_from_metadata_instead_of_module_name_heuristics() {
    let properties = serde_json::from_value(serde_json::json!({
        "xidl_imports": [
            { "prefix": "models", "versions": [] },
            { "prefix": "billing", "versions": ["current", "v2"] }
        ]
    }))
    .unwrap();
    let units = DocumentUnits::of(&properties);
    for (segments, expected) in [
        (
            vec!["models", "v2", "Invoice"],
            "openapi_models.json#/components/schemas/v2.Invoice",
        ),
        (
            vec!["models", "common", "Invoice"],
            "openapi_models.json#/components/schemas/common.Invoice",
        ),
        (
            vec!["billing", "current", "Invoice"],
            "openapi_billing.current.json#/components/schemas/current.Invoice",
        ),
        (
            vec!["billing", "v2", "Invoice"],
            "openapi_billing.v2.json#/components/schemas/v2.Invoice",
        ),
    ] {
        let name = ScopedName {
            is_root: false,
            name: segments.into_iter().map(str::to_owned).collect(),
        };
        assert_eq!(units.reference(&name).as_deref(), Some(expected));
    }
}
