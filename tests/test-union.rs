#[test]
fn test_union_def() {
    let ast = idl_rs::parser::parser_text(
        r#"
        union A;
        union B switch (int32) {};
    "#,
    )
    .unwrap();
    insta::assert_debug_snapshot!(ast)
}
