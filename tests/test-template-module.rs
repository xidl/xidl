#[test]
fn test_template_module_dcl() {
    let ast = idl_rs::parser::parser_text(
        r#"
        module MyTemplModule <typename T, struct S> {
        };

        module MyTemplModule <typename T, struct S, ::A a, A::B::C::D a> {
        };

        module MyTemplModule <typename T, struct S, long m> {
            alias MyTemplModule<T2, S2, m> MyTemplModule;
            interface Bar : A::Foo {};
        };
    "#,
    )
    .unwrap();
    insta::assert_debug_snapshot!(ast)
}
