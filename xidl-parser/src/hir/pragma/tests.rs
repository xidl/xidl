use super::*;
use crate::typed_ast::{PreprocArg, PreprocCall, PreprocDirective};

fn pragma(argument: &str) -> PreprocCall {
    PreprocCall {
        directive: PreprocDirective("#pragma".to_string()),
        argument: Some(PreprocArg(argument.to_string())),
    }
}

#[test]
fn parses_supported_xidlc_pragmas() {
    assert!(matches!(
        parse_xidlc_pragma(&pragma("xidlc package \"demo.pkg\"")),
        Some(Pragma::XidlcPackage(value)) if value == "demo.pkg"
    ));
    assert!(matches!(
        parse_xidlc_pragma(&pragma("xidlc version '3.1.0'")),
        Some(Pragma::XidlcOpenApiVersion(value)) if value == "3.1.0"
    ));
    assert!(matches!(
        parse_xidlc_pragma(&pragma("xidlc service \"https://demo.test\" \"Demo API\"")),
        Some(Pragma::XidlcOpenApiService { base_url, description })
            if base_url == "https://demo.test" && description.as_deref() == Some("Demo API")
    ));
    assert!(matches!(
        parse_xidlc_pragma(&pragma("xidlc service https://demo.test Demo")),
        Some(Pragma::XidlcOpenApiService { base_url, description })
            if base_url == "https://demo.test" && description.as_deref() == Some("Demo")
    ));
    assert!(matches!(
        parse_xidlc_pragma(&pragma("xidlc openapi version \"3.0.3\"")),
        Some(Pragma::XidlcOpenApiVersion(value)) if value == "3.0.3"
    ));
    assert!(matches!(
        parse_xidlc_pragma(&pragma("xidlc openapi service https://demo.test")),
        Some(Pragma::XidlcOpenApiService { base_url, description })
            if base_url == "https://demo.test" && description.is_none()
    ));
}

#[test]
fn rejects_non_xidlc_or_incomplete_pragmas() {
    assert!(
        parse_xidlc_pragma(&PreprocCall {
            directive: PreprocDirective("#define".to_string()),
            argument: Some(PreprocArg("xidlc package demo".to_string())),
        })
        .is_none()
    );
    assert!(matches!(
        parse_xidlc_pragma(&pragma("other package demo")),
        Some(Pragma::Custom(CustomPragma { directive, argument }))
            if directive == "#pragma" && argument.as_deref() == Some("other package demo")
    ));
    assert!(matches!(
        parse_xidlc_pragma(&pragma("xidlc")),
        Some(Pragma::Custom(CustomPragma { directive, argument }))
            if directive == "#pragma" && argument.as_deref() == Some("xidlc")
    ));
    assert!(matches!(
        parse_xidlc_pragma(&pragma("xidlc package")),
        Some(Pragma::Custom(CustomPragma { directive, argument }))
            if directive == "#pragma" && argument.as_deref() == Some("xidlc package")
    ));
    assert!(matches!(
        parse_xidlc_pragma(&pragma("xidlc openapi")),
        Some(Pragma::Custom(CustomPragma { directive, argument }))
            if directive == "#pragma" && argument.as_deref() == Some("xidlc openapi")
    ));
    assert!(matches!(
        parse_xidlc_pragma(&pragma("xidlc service")),
        Some(Pragma::Custom(CustomPragma { directive, argument }))
            if directive == "#pragma" && argument.as_deref() == Some("xidlc service")
    ));
    assert!(matches!(
        parse_xidlc_pragma(&pragma("xidlc unknown(value)")),
        Some(Pragma::Custom(CustomPragma { directive, argument }))
            if directive == "#pragma" && argument.as_deref() == Some("xidlc unknown(value)")
    ));
    assert!(matches!(
        parse_xidlc_pragma(&PreprocCall {
            directive: PreprocDirective("#pragma".to_string()),
            argument: None,
        }),
        Some(Pragma::Custom(CustomPragma { directive, argument }))
            if directive == "#pragma" && argument.is_none()
    ));
}

#[test]
fn trims_quoted_and_unquoted_values() {
    assert_eq!(trim_pragma_value("\"demo\""), "demo");
    assert_eq!(trim_pragma_value("'demo'"), "demo");
    assert_eq!(trim_pragma_value(" demo "), "demo");
}

#[test]
fn rejects_invalid_openapi_tokens() {
    assert!(matches!(
        parse_xidlc_pragma(&pragma("xidlc openapi unknown value")),
        Some(Pragma::Custom(CustomPragma { directive, argument }))
            if directive == "#pragma" && argument.as_deref() == Some("xidlc openapi unknown value")
    ));
}
