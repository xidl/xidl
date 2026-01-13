#[test]
fn test_preproc() {
    let ast = idl_rs::parser::parser_text(
        r#"
        #include "aaaa"

        #ifdef BASIC

        module A {};

        #program once

        module A {}; #endif
    "#,
    )
    .unwrap();
    insta::assert_debug_snapshot!(ast)
}
