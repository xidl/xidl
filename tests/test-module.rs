const TEST_CASES: &[(&str, &str)] = &[(
    "module_dcl",
    r#"
        module A {};

        module B {
        const u8 a = 10;
        struct B;
        };
    "#,
)];

#[test]
fn test_typed_ast() {
    for (name, text) in TEST_CASES {
        let ast = idl_rs::parser::parser_text(text).unwrap();
        let snapshot = format!("typed_ast__{name}");
        insta::assert_debug_snapshot!(snapshot, ast);
    }
}

#[test]
fn test_hir() {
    for (name, text) in TEST_CASES {
        let typed = idl_rs::parser::parser_text(text).unwrap();
        let hir = idl_rs::hir::Specification::from(typed);
        let snapshot = format!("hir__{name}");
        insta::assert_debug_snapshot!(snapshot, hir);
    }
}
