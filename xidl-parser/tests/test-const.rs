const TEST_CASES: &[(&str, &str)] = &[
    ("const_dec", "const int32 a = 10;"),
    ("const_binary", "const int32 a = 0b10;"),
    ("const_oct", "const int32 a = 0o10;"),
    ("const_hex", "const int32 a = 0xff;"),
    (
        "const_scoped_name",
        r#"
            const int a = 0;
            const uint8 a = 0;
            const uint16 a = 0;
            const uint32 a = 0;
            const uint64 a = 0;

            const int8 a = 0;
            const int16 a = 0;
            const int32 a = 0;
            const int64 a = 0;

            const char8 a = 0;
            const char16 a = 0;

            const char C1 = 'X';
            const wchar C2 = L'X';
            const string C3 = "aaa";
            const wstring C3 = L"aaa";
            const bool C3 = false;

            const ::A a = 0;
            const A::B a = 0;
            const A::B::C a = 0;
            const A::B::C::D::E::F a = 0;

            const M::Size MYSIZE = M::medium;

            const float const_float = 13.1;
            const double const_double = 84.1e;
            const long double const_longdouble = 46.1;
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
fn test_hir() {
    for (name, text) in TEST_CASES {
        let typed = xidl_parser::parser::parser_text(text).unwrap();
        let hir = xidl_parser::hir::Specification::from(typed);
        let snapshot = format!("hir__{name}");
        insta::assert_debug_snapshot!(snapshot, hir);
    }
}
