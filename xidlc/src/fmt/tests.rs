use super::format_idl_source;

#[test]
fn idl_formatter_preserves_passthrough_names_arrays_and_quoted_brackets() {
    let source = r#"@rust-derive(Debug) @photon_audit(data=["x", "y"])
        struct Item { @rename("a\\b[0]") string id; };"#;
    let formatted = format_idl_source(source).expect("format valid annotations");
    assert!(formatted.contains("@rust-derive(Debug)"), "{formatted}");
    assert!(formatted.contains(r#"data=["x", "y"]"#), "{formatted}");
    assert!(formatted.contains(r#"@rename("a\\b[0]")"#), "{formatted}");
    assert_eq!(format_idl_source(&formatted).unwrap(), formatted);
    assert!(format_idl_source("struct Broken {").is_err());
}

#[test]
fn idl_formatter_appends_trailing_newline() {
    let source = "struct Point {\n    int32 x;\n};";
    let formatted = format_idl_source(source).expect("format idl");
    assert!(formatted.ends_with('\n'));
}
