use super::{integer, project, values};
use crate::hir::{ParserProperties, Specification};

#[test]
fn constants_use_idl_precision_and_repeated_lowering_is_stable() {
    for (source, expected) in [
        ("const unsigned long VALUE = ~0xffffffff;", 0),
        ("const long VALUE = ~0xffffffff;", 0),
        ("const unsigned long VALUE = ~0;", 4_294_967_295),
        (
            "const unsigned long long VALUE = ~0;",
            18_446_744_073_709_551_615,
        ),
        ("const long long VALUE = ~0xffffffffffffffff;", 0),
        (
            "const long long VALUE = -9223372036854775808;",
            -9_223_372_036_854_775_808,
        ),
        ("const long VALUE = ~(-2);", 1),
        ("const octet VALUE = ~0xffffff00;", 255),
        ("const long VALUE = (-2 >> 31);", 1),
        ("const unsigned long VALUE = +2;", 2),
    ] {
        let mut spec = project(source, ParserProperties::new());
        assert_eq!(values(&spec)["VALUE"], integer(expected), "{source}");
        let before = serde_json::to_value(&spec).unwrap();
        super::super::IntegerDeclarations::normalize(&mut spec).expect("repeat lowering");
        assert_eq!(serde_json::to_value(&spec).unwrap(), before, "{source}");
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
            interface Limits {
                const Word AVAILABLE = ~MASK;
            };
        };
        "#,
        ParserProperties::new(),
    );
    let values = values(&spec);
    assert_eq!(values["LOCAL"], integer(3));
    assert_eq!(values["ROOT"], integer(2));
    assert_eq!(values["AVAILABLE"], integer(3));
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
fn invalid_local_constants_name_the_declaration_and_integer_contract() {
    for (declaration, reason) in [
        ("const short BROKEN = 65536;", "16-bit signed"),
        ("const unsigned long BROKEN = -2;", "32-bit unsigned"),
        (
            "const long BROKEN = 4294967296 - 4294967294;",
            "32-bit unsigned",
        ),
        ("const long BROKEN = 1 / 0;", "invalid or overflows"),
        ("const long BROKEN = 1 << 64;", "invalid or overflows"),
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
