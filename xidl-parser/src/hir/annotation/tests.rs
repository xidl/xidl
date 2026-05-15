use super::*;

fn scoped(name: &str, params: Option<AnnotationParams>) -> Annotation {
    Annotation::ScopedName {
        name: ScopedName {
            name: vec![name.to_string()],
            is_root: false,
        },
        params,
    }
}

fn string_expr(value: &str) -> ConstExpr {
    ConstExpr::Literal(Literal::StringLiteral(value.to_string()))
}

#[test]
fn serialize_annotation_helpers_read_standard_param_forms() {
    let annotations = vec![
        scoped(
            "rename",
            Some(AnnotationParams::ConstExpr(string_expr("\"wireName\""))),
        ),
        scoped(
            "serialize_name",
            Some(AnnotationParams::ConstExpr(string_expr("\"writeOnly\""))),
        ),
        scoped(
            "deserialize_name",
            Some(AnnotationParams::Positional(vec![
                string_expr("\"readOnly\""),
                string_expr("\"legacyName\""),
            ])),
        ),
        scoped(
            "rename_all",
            Some(AnnotationParams::Params(vec![AnnotationParam {
                ident: "rule".to_string(),
                value: Some(string_expr("\"camelCase\"")),
            }])),
        ),
    ];

    assert_eq!(field_rename(&annotations), Some("wireName".to_string()));
    assert_eq!(serialize_name(&annotations), Some("writeOnly".to_string()));
    assert_eq!(deserialize_name(&annotations), Some("readOnly".to_string()));
    assert_eq!(
        deserialize_aliases(&annotations),
        vec!["legacyName".to_string()]
    );
    assert_eq!(rename_all(&annotations), Some("camelCase".to_string()));
}

#[test]
fn normalize_annotation_params_supports_bare_raw_values() {
    let params = normalize_annotation_params(&AnnotationParams::Raw("\"camelCase\"".to_string()));
    assert_eq!(params.get("value").map(String::as_str), Some("camelCase"));
}

#[test]
fn effective_wire_name_applies_container_rename_all() {
    let container = vec![scoped(
        "rename_all",
        Some(AnnotationParams::ConstExpr(string_expr("\"camelCase\""))),
    )];
    assert_eq!(
        effective_wire_name("user_id", &[], &container),
        "userId".to_string()
    );

    let explicit = vec![scoped(
        "rename",
        Some(AnnotationParams::ConstExpr(string_expr("\"wireName\""))),
    )];
    assert_eq!(
        effective_wire_name("user_id", &explicit, &container),
        "wireName".to_string()
    );
}
