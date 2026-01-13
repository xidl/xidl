#[test]
fn test_typedef_dcl() {
    let ast = idl_rs::parser::parser_text(
        r#"
        typedef sequence<Foo> FooSeq;
        typedef u8 uint8_t;
        typedef string u8string;
        typedef wstring u16string;
    "#,
    )
    .unwrap();
    insta::assert_debug_snapshot!(ast)
}
