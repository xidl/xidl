#[test]
fn test_bitmask_def() {
    let ast = idl_rs::parser::parser_text(
        r#"
        bitmask A {};
        bitmask A { A, B, C};
        bitmask A { A, B, C,};
    "#,
    )
    .unwrap();
    insta::assert_debug_snapshot!(ast)
}
