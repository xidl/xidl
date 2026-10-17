const TEST_CASES: &[(&str, &str)] = &[
    (
        "import",
        r#"
        import "timestamp";

        struct Event {
            timestamp::v1::Instant at;
        };
    "#,
    ),
    (
        "import_version",
        r#"
        import "timestamp/util.idl@v1";

        struct Event {
            util::Id id;
        };
    "#,
    ),
];

#[test]
fn test_typed_ast() {
    for (name, text) in TEST_CASES {
        let ast = xidl_parser::parser::parser_text(text).unwrap();
        let snapshot = format!("typed_ast__{name}");
        insta::assert_debug_snapshot!(snapshot, ast);
    }
}

#[test]
fn test_hir_ignores_imports() {
    for (name, text) in TEST_CASES {
        let typed = xidl_parser::parser::parser_text(text).unwrap();
        let hir = xidl_parser::hir::Specification::from(typed);
        let snapshot = format!("hir__{name}");
        insta::assert_debug_snapshot!(snapshot, hir);
    }
}
