#[test]
fn test_bitset_def() {
    let ast = idl_rs::parser::parser_text(
        r#"
        bitset A {};
        bitset A: B {};
        bitset A: B {
            bitfield<1> a;
            bitfield<1> a b c;
        };
    "#,
    )
    .unwrap();
    insta::assert_debug_snapshot!(ast)
}
