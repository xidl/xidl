const TEST_CASES: &[(&str, &str)] = &[(
    "except_dcl",
    r#"
        exception HelloWorld {
            u8 a;
            u16 b[10];
            string c[10][20];
            sequence<u8> c;
            string<20> d;
            wstring<20> d;
            fixed<1,2> d;
        };
    "#,
)];

#[test]
fn test_typed_ast() {
    for (name, text) in TEST_CASES {
        let ast = xidl_parser::parser::parser_text(text).unwrap();
        let snapshot = format!("typed_ast__{name}");
        insta::assert_debug_snapshot!(snapshot, ast);
    }
}

#[test]
fn test_hir() {
    for (name, text) in TEST_CASES {
        let typed = xidl_parser::parser::parser_text(text).unwrap();
        let hir = xidl_parser::hir::Specification::from(typed);
        let snapshot = format!("hir__{name}");
        insta::assert_debug_snapshot!(snapshot, hir);
    }
}
