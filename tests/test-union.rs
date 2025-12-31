#[test]
fn test_union_def() {
    let ast = idl_rs::parser::parser_text(
        r#"
        union A;
        union B switch (int32) {};
        union C switch (int32) {
            case 0:
                int32 a;
            case 1:
                string b;
        };
    "#,
    )
    .unwrap();
    insta::assert_debug_snapshot!(ast)
}
