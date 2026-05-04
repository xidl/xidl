use super::*;
use crate::hir::{
    Annotation, AnnotationParams, OpDcl, OpTypeSpec, ParamDcl, ParameterDcls, SimpleDeclarator,
    StringType, TypeSpec,
};
use crate::http_hir::HttpMethod;

fn builtin(name: &str, raw: &str) -> Annotation {
    Annotation::Builtin {
        name: name.to_string(),
        params: Some(AnnotationParams::Raw(raw.to_string())),
    }
}

fn param(name: &str, annotations: Vec<Annotation>) -> ParamDcl {
    ParamDcl {
        annotations,
        attr: None,
        ty: TypeSpec::StringType(StringType { bound: None }),
        declarator: SimpleDeclarator(name.to_string()),
    }
}

#[test]
fn project_params_infers_kinds_and_tracks_bindings() {
    let op = OpDcl {
        annotations: Vec::new(),
        ty: OpTypeSpec::Void,
        ident: "get_city".to_string(),
        parameter: Some(ParameterDcls(vec![
            param("id", vec![builtin("path", "\"cityId\"")]),
            param("region", vec![builtin("query", "\"r\"")]),
            param("payload", vec![builtin("flatten", "")]),
        ])),
        raises: None,
    };
    let (request, response, path_counts, query_counts) = project_params(
        &op,
        HttpMethod::Post,
        None,
        &[std::collections::HashSet::from(["cityId".to_string()])],
        &[std::collections::HashSet::from(["r".to_string()])],
    )
    .expect("project params");
    assert!(response.is_empty());
    assert_eq!(request[0].kind, HttpParamKind::Path);
    assert_eq!(request[1].kind, HttpParamKind::Query);
    assert_eq!(request[2].kind, HttpParamKind::Body);
    assert!(request[2].flatten);
    assert_eq!(path_counts.get("cityId"), Some(&1));
    assert_eq!(query_counts.get("r"), Some(&1));
}
