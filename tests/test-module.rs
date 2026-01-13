#[test]
fn test_module_dcl() {
    let ast = idl_rs::parser::parser_text(
        r#"
        module A {};

        module B {
        const u8 a = 10;
        struct B;
        };
    "#,
    )
    .unwrap();
    insta::assert_debug_snapshot!(ast)
}
