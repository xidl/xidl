#[test]
fn test_interface() {
    let ast = idl_rs::parser::parser_text(
        r#"
        interface HelloWorld;
        interface HelloWorld {};

        interface HelloWorld: Parent {};
        interface HelloWorld: Parent1, Parent2, Parent3 {};

        interface A: B, C, D {
            void func1();
            void func1() raises(A);
            void func1() raises(A,B,C);
            void func1(in u8 attr, out u16 attr);
            void func1(in u8 attr, out u16 attr) raises(A);
            void func1(in u8 attr, out u16 attr) raises(A,B,C);
        };


    "#,
    )
    .unwrap();
    insta::assert_debug_snapshot!(ast)
}
