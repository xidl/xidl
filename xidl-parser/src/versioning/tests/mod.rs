use crate::hir::{Annotation, Enumerator, ParserProperties, ScopedName, Specification};
use serde_json::Value;

mod analyze;
mod carriers;
mod error;
mod lifecycle;
mod project;
mod timeline;

/// Lowers IDL text into source HIR without interface expansion.
pub(super) fn parse(text: &str) -> Specification {
    let typed = crate::parser::parser_text(text).expect("IDL should parse");
    Specification::from_typed_ast_with_properties_and_path(typed, test_properties(), "test.idl")
        .expect("IDL should lower")
}

/// Lowers IDL text, expecting a failure.
pub(super) fn lower_error(text: &str) -> crate::error::ParseError {
    let typed = crate::parser::parser_text(text).expect("IDL should parse");
    match Specification::from_typed_ast_with_properties_and_path(
        typed,
        test_properties(),
        "test.idl",
    ) {
        Err(error) => error,
        Ok(_) => panic!("expected lowering to fail"),
    }
}

/// Properties that keep the lowering focused on versioning.
pub(super) fn test_properties() -> ParserProperties {
    ParserProperties::from([("expand_interface".to_string(), Value::Bool(false))])
}

/// Analyses IDL text that must declare a version timeline.
pub(super) fn analyze(text: &str) -> (Specification, crate::versioning::VersioningRecord) {
    let spec = parse(text);
    let record = crate::versioning::analyze(&spec)
        .expect("analysis should succeed")
        .expect("specification should be versioned");
    (spec, record)
}

/// Builds an enum member with an optional `@rename` display value.
pub(super) fn member(ident: &str, value: Option<&str>) -> Enumerator {
    Enumerator {
        annotations: value
            .map(|value| {
                vec![Annotation::Rename {
                    name: value.to_string(),
                }]
            })
            .unwrap_or_default(),
        ident: ident.to_string(),
    }
}

/// Builds an `Enum::member` version reference.
pub(super) fn reference(enum_ident: &str, member: &str) -> ScopedName {
    ScopedName {
        name: vec![enum_ident.to_string(), member.to_string()],
        is_root: false,
    }
}

/// Builds `@added(Enum::member)`.
pub(super) fn added(enum_ident: &str, member: &str) -> Annotation {
    Annotation::Added {
        version: reference(enum_ident, member),
    }
}

/// Builds `@removed(Enum::member)`.
pub(super) fn removed(enum_ident: &str, member: &str) -> Annotation {
    Annotation::Removed {
        version: reference(enum_ident, member),
    }
}

/// Builds generation properties from string pairs.
pub(super) fn properties(pairs: &[(&str, &str)]) -> ParserProperties {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_string(), Value::String((*value).to_string())))
        .collect()
}
