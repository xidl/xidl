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

#[test]
fn normalize_annotation_params_covers_raw_named_and_expr_forms() {
    let raw = AnnotationParams::Raw(
        r#"Name="wire,value", enabled='true', escaped="a\,b", bare"#.to_string(),
    );
    let normalized = normalize_annotation_params(&raw);
    assert_eq!(
        normalized.get("name").map(String::as_str),
        Some("wire,value")
    );
    assert_eq!(normalized.get("enabled").map(String::as_str), Some("true"));
    assert_eq!(normalized.get("escaped").map(String::as_str), Some("a,b"));
    assert_eq!(normalized.get("value").map(String::as_str), Some("bare"));

    let params = AnnotationParams::Params(vec![
        AnnotationParam {
            ident: "Named".to_string(),
            value: Some(ConstExpr::Literal(Literal::StringLiteral(
                "\"alias\"".to_string(),
            ))),
        },
        AnnotationParam {
            ident: "Missing".to_string(),
            value: None,
        },
    ]);
    let normalized = normalize_annotation_params(&params);
    assert_eq!(normalized.get("named").map(String::as_str), Some("alias"));
    assert_eq!(normalized.get("missing").map(String::as_str), Some(""));

    let positional = AnnotationParams::Positional(vec![
        ConstExpr::Literal(Literal::IntegerLiteral(IntegerLiteral("1".to_string()))),
        ConstExpr::Literal(Literal::IntegerLiteral(IntegerLiteral("2".to_string()))),
    ]);
    assert_eq!(
        normalize_annotation_params(&positional)
            .get("value")
            .map(String::as_str),
        Some("1, 2")
    );
}

#[test]
fn rename_rules_cover_supported_case_conversions() {
    let cases = [
        (RenameRule::None, "User ID", "User ID", "None"),
        (RenameRule::LowerCase, "User ID", "userid", "lowercase"),
        (RenameRule::UpperCase, "User ID", "USERID", "UPPERCASE"),
        (RenameRule::PascalCase, "user_id", "UserId", "PascalCase"),
        (RenameRule::CamelCase, "user_id", "userId", "camelCase"),
        (RenameRule::SnakeCase, "User ID", "user_id", "snake_case"),
        (
            RenameRule::ScreamingSnakeCase,
            "User ID",
            "USER_ID",
            "SCREAMING_SNAKE_CASE",
        ),
        (RenameRule::KebabCase, "User ID", "user-id", "kebab-case"),
        (
            RenameRule::ScreamingKebabCase,
            "User ID",
            "USER-ID",
            "SCREAMING-KEBAB-CASE",
        ),
    ];

    for (rule, raw, expected, rendered) in cases {
        assert_eq!(apply_rename_rule(raw, rule.clone()), expected);
        assert_eq!(rule.as_str(), rendered);
        assert_eq!(rendered.parse::<RenameRule>().expect("rename rule"), rule);
    }

    assert_eq!(
        "unknown".parse::<RenameRule>().expect("unknown rule"),
        RenameRule::None
    );
    assert_eq!(
        "SCREAMINGSNAKECASE"
            .parse::<RenameRule>()
            .expect("legacy screaming snake"),
        RenameRule::ScreamingSnakeCase
    );
}

#[test]
fn render_annotation_const_expr_covers_literals_and_operators() {
    let scoped = ConstExpr::ScopedName(ScopedName {
        name: vec!["pkg".to_string(), "Value".to_string()],
        is_root: true,
    });
    assert_eq!(render_annotation_const_expr(&scoped), "::pkg::Value");

    let literals = [
        (
            Literal::FloatingPtLiteral(FloatingPtLiteral {
                sign: Some(IntegerSign::Minus),
                integer: DecNumber("12".to_string()),
                fraction: DecNumber("34".to_string()),
            }),
            "-12.34",
        ),
        (Literal::CharLiteral("'c'".to_string()), "'c'"),
        (Literal::WideCharacterLiteral("L'c'".to_string()), "L'c'"),
        (Literal::WideStringLiteral("L\"s\"".to_string()), "L\"s\""),
        (Literal::BooleanLiteral(true), "true"),
    ];
    for (literal, expected) in literals {
        assert_eq!(
            render_annotation_const_expr(&ConstExpr::Literal(literal)),
            expected
        );
    }

    let one = ConstExpr::Literal(Literal::IntegerLiteral(IntegerLiteral("1".to_string())));
    let two = ConstExpr::Literal(Literal::IntegerLiteral(IntegerLiteral("2".to_string())));
    assert_eq!(
        render_annotation_const_expr(&ConstExpr::UnaryExpr(
            UnaryOperator::Not,
            Box::new(one.clone()),
        )),
        "(~1)"
    );

    let operators = [
        (BinaryOperator::Or, "(1 | 2)"),
        (BinaryOperator::Xor, "(1 ^ 2)"),
        (BinaryOperator::And, "(1 & 2)"),
        (BinaryOperator::LeftShift, "(1 << 2)"),
        (BinaryOperator::RightShift, "(1 >> 2)"),
        (BinaryOperator::Add, "(1 + 2)"),
        (BinaryOperator::Sub, "(1 - 2)"),
        (BinaryOperator::Mult, "(1 * 2)"),
        (BinaryOperator::Div, "(1 / 2)"),
        (BinaryOperator::Mod, "(1 % 2)"),
    ];
    for (operator, expected) in operators {
        assert_eq!(
            render_annotation_const_expr(&ConstExpr::BinaryExpr(
                operator,
                Box::new(one.clone()),
                Box::new(two.clone()),
            )),
            expected
        );
    }
}

#[test]
fn annotation_from_typed_ast_recognizes_builtin_aliases_and_extras() {
    fn raw_builtin(name: &str, raw: &str) -> crate::typed_ast::AnnotationAppl {
        crate::typed_ast::AnnotationAppl {
            name: crate::typed_ast::AnnotationName::Builtin(name.to_string()),
            params: Some(crate::typed_ast::AnnotationParams::Raw(raw.to_string())),
            builtin: None,
            is_extend: false,
            extra: Vec::new(),
        }
    }

    assert!(matches!(
        Annotation::from(raw_builtin("rename", r#""wire""#)),
        Annotation::Rename { name } if name == "wire"
    ));
    assert!(matches!(
        Annotation::from(raw_builtin("name", r#"name="alias""#)),
        Annotation::Rename { name } if name == "alias"
    ));
    assert!(matches!(
        Annotation::from(raw_builtin("rename_all", r#"rule="snake_case""#)),
        Annotation::RenameAll {
            rule: RenameRule::SnakeCase
        }
    ));
    assert!(matches!(
        Annotation::from(raw_builtin("skip", "")),
        Annotation::Skip
    ));

    let expanded = expand_annotations(vec![crate::typed_ast::AnnotationAppl {
        name: crate::typed_ast::AnnotationName::Builtin("outer".to_string()),
        params: None,
        builtin: None,
        is_extend: false,
        extra: vec![raw_builtin("skip", "")],
    }]);
    assert_eq!(expanded.len(), 2);
    assert!(is_skipped(&expanded));
}

#[test]
fn annotation_metadata_helpers_cover_builtin_scoped_and_other_variants() {
    let builtin = Annotation::Builtin {
        name: "produce".to_string(),
        params: Some(AnnotationParams::Raw(r#""application/json""#.to_string())),
    };
    assert_eq!(annotation_name(&builtin), Some("produce"));
    assert!(annotation_params(&builtin).is_some());

    let scoped = Annotation::ScopedName {
        name: ScopedName {
            name: vec!["idl".to_string(), "custom".to_string()],
            is_root: false,
        },
        params: Some(AnnotationParams::Raw("enabled=true".to_string())),
    };
    assert_eq!(annotation_name(&scoped), Some("custom"));
    assert!(annotation_params(&scoped).is_some());

    assert_eq!(annotation_name(&Annotation::Skip), None);
    assert!(annotation_params(&Annotation::Skip).is_none());
    assert_eq!(
        annotation_id_value(&[Annotation::Id {
            value: "bad".to_string()
        }]),
        None
    );
    assert_eq!(
        effective_wire_name("raw_name", &[], &[]),
        "raw_name".to_string()
    );
}
