use super::*;
use crate::hir::{
    Annotation, AnnotationParams, OpDcl, OpTypeSpec, ParamAttribute, ParamDcl, ParameterDcls,
    ReadonlyAttrDeclarator, ReadonlyAttrSpec, SimpleDeclarator, StringType, TypeSpec,
};

fn builtin(name: &str, raw: &str) -> Annotation {
    Annotation::Builtin {
        name: name.to_string(),
        params: Some(AnnotationParams::Raw(raw.to_string())),
    }
}

fn string_param(name: &str, annotations: Vec<Annotation>, attr: Option<&str>) -> ParamDcl {
    ParamDcl {
        annotations,
        attr: attr.map(|value| ParamAttribute(value.to_string())),
        ty: TypeSpec::StringType(StringType { bound: None }),
        declarator: SimpleDeclarator(name.to_string()),
    }
}

fn op(name: &str, params: Vec<ParamDcl>) -> OpDcl {
    OpDcl {
        annotations: Vec::new(),
        ty: OpTypeSpec::Void,
        ident: name.to_string(),
        parameter: Some(ParameterDcls(params)),
        raises: None,
    }
}

#[test]
fn explicit_param_binding_detects_conflicts_and_defaults_name() {
    let param = string_param("city_id", vec![builtin("path", "")], None);
    let binding = explicit_param_binding(&param).expect("path binding");
    assert_eq!(binding.expect("binding").source, HttpParamKind::Path);

    let conflict = string_param(
        "city_id",
        vec![builtin("path", ""), builtin("query", "")],
        None,
    );
    let err = explicit_param_binding(&conflict)
        .err()
        .expect("conflicting binding");
    assert!(err.contains("conflicting source annotations"));
}

#[test]
fn route_from_annotations_deduplicates_paths_and_rejects_multiple_verbs() {
    let annotations = vec![
        builtin("get", "path=\"//cities//{id}/\""),
        builtin("path", "path=\"/cities/{id}\""),
        builtin("path", "path=\"/cities/{id}\""),
    ];
    let (method, paths) = route_from_annotations(&annotations, HttpMethod::Post).expect("route");
    assert_eq!(method, HttpMethod::Get);
    assert_eq!(paths, vec!["/cities/{id}".to_string()]);

    let err = route_from_annotations(
        &[builtin("get", "\"/one\""), builtin("post", "\"/two\"")],
        HttpMethod::Post,
    )
    .expect_err("multiple verbs");
    assert!(err.contains("more than one HTTP verb annotation"));
}

#[test]
fn auto_default_method_path_uses_only_request_side_path_bindings() {
    let op = op(
        "watch_city",
        vec![
            string_param(
                "id",
                vec![
                    builtin("path", ""),
                    Annotation::Rename {
                        name: "cityId".to_string(),
                    },
                ],
                Some("in"),
            ),
            string_param(
                "etag",
                vec![
                    builtin("path", ""),
                    Annotation::Rename {
                        name: "etag".to_string(),
                    },
                ],
                Some("out"),
            ),
            string_param("region", Vec::new(), Some("in")),
        ],
    );
    let path = auto_default_method_path(&op, HttpMethod::Get).expect("default path");
    assert_eq!(path, "/watch_city/{cityId}");
}

#[test]
fn parse_route_template_handles_query_and_catch_all_variants() {
    let route = parse_route_template("/files/{*rest}{?cursor,limit}").expect("route");
    assert_eq!(route.path, "/files/{*rest}");
    assert_eq!(route.path_params, vec!["rest".to_string()]);
    assert_eq!(
        route.query_params,
        vec!["cursor".to_string(), "limit".to_string()]
    );

    let unmatched = parse_route_template("/files/{rest").expect_err("unmatched brace");
    assert!(unmatched.contains("unmatched '{'"));

    let empty_query = parse_route_template("/files{?}").expect_err("empty query");
    assert!(empty_query.contains("at least one variable"));

    let catch_all = parse_route_template("/files/{*rest}/tail").expect_err("bad catch-all");
    assert!(catch_all.contains("catch-all variable must be at the end"));
}

#[test]
fn route_helpers_normalize_and_format_paths() {
    assert_eq!(normalize_path(" //cities/// "), "/cities");
    assert_eq!(normalize_path(""), "/");
    assert_eq!(attribute_path("status"), "/attribute/status");
    assert_eq!(
        default_path(&["api".to_string(), "v1".to_string()], "CityApi", "list"),
        "/api/v1/CityApi/list"
    );
    assert_eq!(
        operation_id(&["api".to_string(), "v1".to_string()], "CityApi", "list"),
        "api.v1.CityApi.list"
    );

    let spec = ReadonlyAttrSpec {
        ty: TypeSpec::StringType(StringType { bound: None }),
        declarator: ReadonlyAttrDeclarator::SimpleDeclarator(SimpleDeclarator("status".into())),
    };
    assert_eq!(readonly_attr_names(&spec), vec!["status".to_string()]);
}
