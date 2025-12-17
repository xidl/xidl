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
        r"
            const A a = 0xff;
            const ::A a = 0;
            const A::B a = 0;
            const ::A::B::C a = 0;
            const M::Size MYSIZE = M::medium;
        ",
    )
    .unwrap();
    insta::assert_debug_snapshot!(ast)
}
