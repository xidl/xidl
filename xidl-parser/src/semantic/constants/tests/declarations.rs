use super::{plans, project, values};
use crate::hir::{ParserProperties, Specification};

#[test]
fn ordinary_constants_retain_expressions_and_resolving_types_is_stable() {
    for source in [
        "const unsigned long VALUE = ~0;",
        "const long long VALUE = -9223372036854775808;",
        "const long VALUE = ~(-0);",
        "const long VALUE = (-2 >> 31);",
        "const octet VALUE = ~0xffffff00;",
        "const unsigned long VALUE = -1 + 2;",
        "const long VALUE = 1 / 0;",
        "const unsigned long VALUE = 0xffffffff + 1;",
    ] {
        let typed = crate::parser::parser_text(source).unwrap();
        let raw = Specification::from_typed_ast_with_path(typed, "constants.idl").unwrap();
        let mut spec = project(source, ParserProperties::new());
        assert_eq!(
            values(&spec),
            values(&raw),
            "ordinary arithmetic stays symbolic: {source}"
        );
        assert!(plans(&spec)["VALUE"].is_object());
        let before = serde_json::to_value(&spec).unwrap();
        super::super::IntegerDeclarations::normalize(&mut spec, &Default::default()).unwrap();
        assert_eq!(serde_json::to_value(&spec).unwrap(), before);
    }
}

#[test]
fn aliases_and_interface_constants_resolve_in_their_declaration_scope() {
    let spec = project(
        r#"
        const unsigned long MASK = 0xfffffffd;
        module policy {
            typedef unsigned long Word;
            const Word MASK = 0xfffffffc;
            const Word LOCAL = ~MASK;
            const Word ROOT = ~::MASK;
            interface Limits { const Word AVAILABLE = ~MASK; };
        };
        "#,
        ParserProperties::new(),
    );
    let plans = plans(&spec);
    assert_eq!(plans["LOCAL"], plans["AVAILABLE"]);
    assert!(plans["LOCAL"].to_string().contains("4294967292"));
    assert!(plans["ROOT"].to_string().contains("4294967293"));
    insta::assert_snapshot!(
        "integer_expression_types",
        serde_json::to_string_pretty(&plans).unwrap()
    );
}

#[test]
fn non_integral_and_external_constants_retain_their_expressions() {
    let source = r#"
        typedef double Rate;
        enum Channel { sms, chat };
        const Channel PINNED = Channel::sms;
        const Rate PRICE = 1.5;
        const string TEXT = "~keep!";
        const char SYMBOL = '~';
        const boolean ENABLED = TRUE;
        const pkg::Word EXTERNAL_TYPE = 2;
        const unsigned long EXTERNAL_VALUE = pkg::MASK;
        const unsigned long LOCAL_DEPENDENCY = EXTERNAL_VALUE + 1;
        const pkg::Channel EXTERNAL_ENUM = pkg::Channel::sms;
    "#;
    let typed = crate::parser::parser_text(source).expect("symbolic constant syntax");
    let raw = Specification::from_typed_ast_with_path(typed, "constants.idl")
        .expect("source constant contract");
    assert_eq!(
        values(&project(source, ParserProperties::new())),
        values(&raw)
    );
}

#[test]
fn invalid_literal_types_and_cycles_name_the_integer_contract() {
    for (declaration, reason) in [
        (
            "const long BROKEN = 4294967296 - 4294967294;",
            "32-bit unsigned",
        ),
        (
            "const unsigned long long BROKEN = 18446744073709551616;",
            "integer constant literal",
        ),
        (
            "const unsigned long long BROKEN = 0x10000000000000000;",
            "integer constant literal",
        ),
        (
            "const long BROKEN = OTHER; const long OTHER = BROKEN;",
            "cycle",
        ),
    ] {
        let source = format!("module policy {{ {declaration} }};");
        let typed = crate::parser::parser_text(&source).expect("integer expression syntax");
        let error = Specification::project_typed_ast_versions(
            typed,
            ParserProperties::new(),
            "constants.idl",
        )
        .expect_err("invalid local integer constant")
        .to_string();
        assert!(error.contains("policy::BROKEN"), "{source}: {error}");
        assert!(error.contains(reason), "{source}: {error}");
    }
}
