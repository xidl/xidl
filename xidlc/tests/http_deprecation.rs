use std::{collections::HashMap, path::Path};

use xidl_parser::{lint, parser::parser_text};

#[test]
fn deprecation_marks_api_operations_without_time_based_removal() {
    let source = r#"
        @deprecated(since="2099-01-01", after="2100-01-01")
        interface Legacy {
            @get(path="/legacy") string read();
            @deprecated(since="2099-02-01")
            @get(path="/override") string overridden();
            @deprecated readonly attribute string status;
        };
        interface Current {
            @get(path="/current") string read();
            @deprecated(after="2000-01-01") @get(path="/old") string old();
        };
    "#;
    assert!(lint::lint(&parser_text(source).unwrap()).is_empty());
    let files = xidlc::driver::Generator::new("openapi".into())
        .generate_from_idl(source, Path::new("deprecation.idl"), HashMap::new())
        .unwrap();
    let document: serde_json::Value = serde_json::from_str(files[0].content()).unwrap();
    for path in ["/legacy", "/override", "/old"] {
        assert_eq!(document["paths"][path]["get"]["deprecated"], true);
    }
    assert_ne!(document["paths"]["/current"]["get"]["deprecated"], true);
    let inherited = document["paths"]["/legacy"]["get"]["description"]
        .as_str()
        .unwrap();
    assert!(inherited.contains("2099-01-01T00:00:00Z"));
    assert!(inherited.contains("2100-01-01T23:59:59Z"));
    let overridden = document["paths"]["/override"]["get"]["description"]
        .as_str()
        .unwrap();
    assert!(overridden.contains("2099-02-01T00:00:00Z"));
    assert!(!overridden.contains("2100"));
}

#[test]
fn deprecation_is_accepted_on_modules_types_members_enumerators_and_parameters() {
    for source in [
        "@deprecated module M { interface I {}; };",
        "@deprecated struct S { @deprecated string value; };",
        "@deprecated union U switch (long) { case 1: long a; };",
        "@deprecated enum E { @deprecated a, b };",
        "@http(410) @deprecated exception E { @deprecated string reason; };",
        "interface I { void read(@deprecated string value); };",
    ] {
        let diagnostics = lint::lint(&parser_text(source).unwrap());
        assert!(diagnostics.is_empty(), "{source}: {diagnostics:?}");
    }
}
