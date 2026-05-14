use super::*;
use crate::hir::{
    Annotation, AnnotationParam, AnnotationParams, ConstExpr, IntegerLiteral, Literal, ScopedName,
};

fn builtin(name: &str, params: Option<AnnotationParams>) -> Annotation {
    Annotation::Builtin {
        name: name.to_string(),
        params,
    }
}

#[test]
fn annotation_helpers_cover_names_presence_and_values() {
    let scoped = Annotation::ScopedName {
        name: ScopedName {
            name: vec!["pkg".to_string(), "flag".to_string()],
            is_root: false,
        },
        params: Some(AnnotationParams::Raw("\"yes\"".to_string())),
    };
    assert_eq!(annotation_name(&scoped), Some("flag"));
    assert!(annotation_params(&scoped).is_some());

    let annotations = vec![builtin("Optional", None), builtin("Consumes", None)];
    assert!(has_annotation(&annotations, "consumes"));
    assert!(has_optional_annotation(&annotations));
    assert!(annotation_params(&Annotation::Final).is_none());
}

#[test]
fn normalize_annotation_params_handles_all_param_forms() {
    let raw = normalize_annotation_params(&AnnotationParams::Raw(
        r#"VALUE="json", Name="X-Trace""#.to_string(),
    ));
    assert_eq!(raw.get("value").map(String::as_str), Some("json"));
    assert_eq!(raw.get("name").map(String::as_str), Some("X-Trace"));

    let params = normalize_annotation_params(&AnnotationParams::Params(vec![AnnotationParam {
        ident: "MediaType".to_string(),
        value: Some(ConstExpr::Literal(Literal::StringLiteral(
            "\"application/json\"".to_string(),
        ))),
    }]));
    assert_eq!(
        params.get("mediatype").map(String::as_str),
        Some("application/json")
    );

    let expr = normalize_annotation_params(&AnnotationParams::ConstExpr(ConstExpr::Literal(
        Literal::IntegerLiteral(IntegerLiteral("7".to_string())),
    )));
    assert_eq!(expr.get("value").map(String::as_str), Some("7"));
}

#[test]
fn media_type_resolution_prefers_method_and_falls_back_to_default() {
    let interface = vec![builtin(
        "Consume",
        Some(AnnotationParams::Raw("\"application/msgpack\"".to_string())),
    )];
    let method = vec![builtin(
        "Consumes",
        Some(AnnotationParams::Raw("\"text/plain\"".to_string())),
    )];
    assert_eq!(
        effective_media_type(&interface, &method, "Consumes"),
        "text/plain"
    );
    assert_eq!(
        effective_media_type(&[], &[], "Produces"),
        "application/json"
    );
    assert_eq!(
        annotation_value(&interface, "Consumes").as_deref(),
        Some("application/msgpack")
    );
    assert_eq!(
        annotation_value(
            &[builtin(
                "Produce",
                Some(AnnotationParams::Raw("\"text/plain\"".to_string()))
            )],
            "Produces"
        )
        .as_deref(),
        Some("text/plain")
    );
    assert!(media_type_annotation_aliases("Other").is_empty());
    assert!(media_type_annotation_matches("X-Codec", "x-codec"));
    assert_eq!(
        annotation_value(
            &[builtin(
                "X-Codec",
                Some(AnnotationParams::Raw("\"application/custom\"".to_string()))
            )],
            "x-codec"
        )
        .as_deref(),
        Some("application/custom")
    );
}
