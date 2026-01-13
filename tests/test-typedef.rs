const TEST_CASES: &[(&str, &str)] = &[(
    "typedef_dcl",
    r#"
        typedef sequence<Foo> FooSeq;
        typedef u8 uint8_t;
        typedef string u8string;
        typedef wstring u16string;
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
