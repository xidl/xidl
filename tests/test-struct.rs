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
    "#,
    )
    .unwrap();
    insta::assert_debug_snapshot!(ast)
}
