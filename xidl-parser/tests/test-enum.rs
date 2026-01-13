const TEST_CASES: &[(&str, &str)] = &[
    ("enum_empty", "enum A { };"),
    ("enum_simple", "enum A { B, C };"),
    ("enum_simple_comma", "enum A { B, C, };"),
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
fn test_hir() {
    for (name, text) in TEST_CASES {
        let typed = xidl_parser::parser::parser_text(text).unwrap();
        let hir = xidl_parser::hir::Specification::from(typed);
        let snapshot = format!("hir__{name}");
        insta::assert_debug_snapshot!(snapshot, hir);
    }
}
