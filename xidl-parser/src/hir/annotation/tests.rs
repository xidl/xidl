use super::*;

#[test]
fn serialize_annotation_helpers_read_standard_param_forms() {
    let annotations = vec![
        Annotation::Rename {
            name: "wireName".to_string(),
        },
        Annotation::RenameAll {
            rule: RenameRule::CamelCase,
        },
    ];

    assert_eq!(field_rename(&annotations), Some("wireName".to_string()));
    assert_eq!(rename_all(&annotations), Some(RenameRule::CamelCase));
}

#[test]
fn normalize_annotation_params_supports_bare_raw_values() {
    let params = normalize_annotation_params(&AnnotationParams::Raw("\"camelCase\"".to_string()));
    assert_eq!(params.get("value").map(String::as_str), Some("camelCase"));
}

#[test]
fn effective_wire_name_applies_container_rename_all() {
    let container = vec![Annotation::RenameAll {
        rule: RenameRule::CamelCase,
    }];
    assert_eq!(
        effective_wire_name("user_id", &[], &container),
        "userId".to_string()
    );

    let explicit = vec![Annotation::Rename {
        name: "wireName".to_string(),
    }];
    assert_eq!(
        effective_wire_name("user_id", &explicit, &container),
        "wireName".to_string()
    );
}
