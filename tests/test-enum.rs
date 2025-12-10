#[test]
fn test_enum_empty() {
    let ast = idl_rs::parser::parser_text("enum A { };").unwrap();
    insta::assert_debug_snapshot!(ast)
}

#[test]
fn test_enum_simple() {
    let ast = idl_rs::parser::parser_text("enum A { B, C };").unwrap();
    insta::assert_debug_snapshot!(ast)
}

#[test]
fn test_enum_simple_comma() {
    let ast = idl_rs::parser::parser_text("enum A { B, C, };").unwrap();
    insta::assert_debug_snapshot!(ast)
}
