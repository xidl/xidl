use std::collections::HashMap;

#[test]
fn rust_codegen_uses_item_level_allow_attrs() {
    let files = xidlc::generate_from_source(
        "rs",
        r#"
        const long foo = 1;

        interface Example {
            void Ping();
        };
        "#,
        HashMap::from([(String::from("enable_metadata"), false.into())]),
    )
    .expect("rust generation should succeed");

    let rendered = files
        .into_iter()
        .find(|file| file.path().ends_with(".rs"))
        .expect("rust generator should emit a rust source file");
    let content = rendered.content();

    assert!(!content.contains("#![allow("));
    assert!(content.contains("#[allow(non_upper_case_globals)]"));
    assert!(content.contains("#[allow(non_snake_case)]"));
    assert!(!content.contains("#[allow(unreachable_patterns)]\n\n"));
}
