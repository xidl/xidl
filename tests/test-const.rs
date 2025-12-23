#[test]
fn test_const_dec() {
    let ast = idl_rs::parser::parser_text("const int32 a = 10;").unwrap();
    insta::assert_debug_snapshot!(ast)
}

#[test]
fn test_const_binary() {
    let ast = idl_rs::parser::parser_text("const int32 a = 0b10;").unwrap();
    insta::assert_debug_snapshot!(ast)
}

#[test]
fn test_const_oct() {
    let ast = idl_rs::parser::parser_text("const int32 a = 0o10;").unwrap();
    insta::assert_debug_snapshot!(ast)
}

#[test]
fn test_const_hex() {
    let ast = idl_rs::parser::parser_text("const int32 a = 0xff;").unwrap();
    insta::assert_debug_snapshot!(ast)
}

#[test]
fn test_const_scoped_name() {
    let ast = idl_rs::parser::parser_text(
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
    )
    .unwrap();
    insta::assert_debug_snapshot!(ast)
}
