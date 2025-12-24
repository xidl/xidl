#[test]
fn test_bitset_def() {
    let ast = idl_rs::parser::parser_text(
        r#"
        bitset A {};
        bitset A: B {};
    "#,
    )
    .unwrap();
    insta::assert_debug_snapshot!(ast)
}
