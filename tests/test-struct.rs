#[test]
fn test_simple() {
    insta::assert_debug_snapshot!(idl_rs::parser::parser_text("").unwrap())
}
