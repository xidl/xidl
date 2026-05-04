use super::*;
use crate::hir::{Annotation, AnnotationParams, ParamAttribute, ScopedName};

fn builtin(name: &str) -> Annotation {
    Annotation::Builtin {
        name: name.to_string(),
        params: None,
    }
}

#[test]
fn semantics_helpers_cover_streams_annotations_and_collisions() {
    assert!(param_is_input(None));
    assert!(!param_is_input(Some(&ParamAttribute("out".to_string()))));
    assert!(param_is_output(Some(&ParamAttribute("out".to_string()))));
    assert!(param_is_output(Some(&ParamAttribute("inout".to_string()))));

    assert_eq!(
        stream_kind(&[builtin("server_stream")]).unwrap(),
        Some(JsonRpcMethodKind::ServerStream)
    );
    assert!(
        stream_kind(&[builtin("server_stream"), builtin("client_stream")])
            .expect_err("conflict")
            .to_string()
            .contains("mutually exclusive")
    );

    let scoped = Annotation::ScopedName {
        name: ScopedName {
            name: vec!["pkg".to_string(), "stream".to_string()],
            is_root: false,
        },
        params: Some(AnnotationParams::Raw("\"yes\"".to_string())),
    };
    assert!(has_annotation(&[scoped], "stream"));

    validate_attr_collision(
        &std::collections::HashSet::new(),
        "status",
        "get_status",
        "",
    )
    .unwrap();
    let user_ops = std::collections::HashSet::from(["get_status"]);
    assert!(
        validate_attr_collision(&user_ops, "status", "get_status", "set_status")
            .expect_err("collision")
            .to_string()
            .contains("conflicts with user-defined operation")
    );
}
