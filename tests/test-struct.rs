#[test]
fn test_struct_empty() {
    let ast = idl_rs::parser::parser_text("").unwrap();
    insta::assert_debug_snapshot!(ast)
}

#[test]
fn test_struct_simple() {
    let ast = idl_rs::parser::parser_text("struct A;").unwrap();
    insta::assert_debug_snapshot!(ast)
}

#[test]
fn test_struct_def() {
    let ast = idl_rs::parser::parser_text(
        r#"
        struct A {};
        struct A {
            int32 a;
        };
        struct A {
            ::A::b a;
        };
        struct A: B {};

        struct _A {};

        struct _Custom {
            Inner var_inner;
        };

        struct HelloWorld {
            u8 a;
            u16 b[10];
            string c[10][20];
            sequence<u8> c;
            string<20> d;
            wstring<20> d;
            // fixed<1,2> d;
            any d;
        };
    "#,
    )
    .unwrap();
    insta::assert_debug_snapshot!(ast)
}
