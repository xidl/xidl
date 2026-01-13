#[test]
fn test_except_dcl() {
    let ast = idl_rs::parser::parser_text(
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
    )
    .unwrap();
    insta::assert_debug_snapshot!(ast)
}
