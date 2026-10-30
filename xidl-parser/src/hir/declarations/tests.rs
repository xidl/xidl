use std::collections::BTreeMap;

use crate::hir::{DeclarationKind, DeclarationNames, FsIncludeResolver, ScopedName, Specification};

#[test]
fn declaration_identity_survives_nested_exports_and_constructed_types() {
    let typed = crate::parser::parser_text(
        r#"
        struct Detail { long code; };
        module ledger {
            typedef struct Detail { string message; } Alias;
            enum State { ready, done };
            union Event switch(long) {
                case 0: struct Payload { string message; } payload;
            };
            interface Api {
                typedef Detail View;
                const long Limit = 2;
                exception Failure { Detail detail; };
            };
        };
    "#,
    )
    .expect("nested declaration syntax");
    let spec = Specification::source_from_typed_ast_with_resolver(
        typed,
        "declarations.idl",
        &mut FsIncludeResolver,
    )
    .expect("nested source declarations");
    let names = DeclarationNames::collect(&spec.0);
    for name in [
        "Detail",
        "ledger::Detail",
        "ledger::Alias",
        "ledger::State::ready",
        "ledger::Event::Payload",
        "ledger::Api::View",
        "ledger::Api::Limit",
        "ledger::Api::Failure",
    ] {
        assert!(names.contains(name), "missing declaration {name}");
    }
    assert!(!names.contains("Payload"));
    let scope = ["ledger".to_string(), "Api".to_string()];
    for (root, expected) in [(false, vec!["ledger", "Detail"]), (true, vec!["Detail"])] {
        let resolved = names
            .resolve(
                &scope,
                &ScopedName {
                    is_root: root,
                    name: vec!["Detail".to_string()],
                },
            )
            .expect("a known declaration");
        assert_eq!(resolved.name, expected);
    }
    let mut roles = BTreeMap::new();
    DeclarationNames::visit_declarations(&spec.0, "", &mut |name, kind| {
        roles.insert(name.to_string(), kind);
    });
    for (name, role) in [
        ("ledger", DeclarationKind::Scope),
        ("ledger::Api", DeclarationKind::Scope),
        ("ledger::Detail", DeclarationKind::Type),
        ("ledger::Alias", DeclarationKind::Type),
        ("ledger::State::ready", DeclarationKind::Member),
        ("ledger::Event::Payload", DeclarationKind::Type),
        ("ledger::Api::Limit", DeclarationKind::Constant),
    ] {
        assert_eq!(roles.get(name), Some(&role), "declaration role for {name}");
    }
}
