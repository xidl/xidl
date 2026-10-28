use super::format_typescript_source;
use crate::error::IdlcError;

#[test]
fn formats_typescript_canonically() {
    let formatted =
        format_typescript_source("const x={a:1};\nlet y=1\n").expect("format valid typescript");
    assert_eq!(formatted, "const x = { a: 1 };\nlet y = 1;\n");
}

#[test]
fn formatting_is_idempotent() {
    let once = format_typescript_source("export   function f(x:number){return x+1}")
        .expect("format valid typescript");
    let twice = format_typescript_source(&once).expect("format formatted typescript");
    assert_eq!(once, twice);
}

#[test]
fn reports_syntax_errors() {
    let error = format_typescript_source("const x =").expect_err("reject invalid typescript");
    assert!(matches!(error, IdlcError::Fmt(_)), "{error:?}");
}
