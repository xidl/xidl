use std::collections::BTreeSet;

use super::{AnnotationDefinition, AnnotationKind, ArgumentShape, BUILTIN_ANNOTATIONS, Site};

#[test]
fn lookup_matches_canonical_case_and_hyphenated_spellings() {
    let definition = AnnotationDefinition::lookup("CLIENT-STREAM")
        .expect("client-stream should resolve through the hyphen alias");
    assert_eq!(definition.name, "client_stream");
    assert_eq!(definition.kind, AnnotationKind::ClientStream);
    assert_eq!(
        AnnotationDefinition::lookup("client_stream").map(|entry| entry.name),
        Some("client_stream")
    );
    assert!(AnnotationDefinition::lookup("unknown-annotation").is_none());
    assert!(AnnotationDefinition::lookup("doc").is_none());
}

#[test]
fn passthrough_matches_target_prefixes_case_insensitively() {
    for name in ["rust-derive", "RUST_derive", "ts-special", "TS_special"] {
        assert!(AnnotationDefinition::is_passthrough(name), "{name}");
    }
    for name in ["relay_audit", "rust", "typescript", ""] {
        assert!(!AnnotationDefinition::is_passthrough(name), "{name}");
    }
}

#[test]
fn placement_rules_distinguish_restricted_and_universal_sites() {
    let optional = AnnotationDefinition::lookup("optional").expect("optional exists");
    assert!(optional.allows(Site::Member));
    assert!(optional.allows(Site::Parameter));
    assert!(!optional.allows(Site::Module));

    let verbatim = AnnotationDefinition::lookup("verbatim").expect("verbatim exists");
    assert!(verbatim.allows(Site::Module));
    assert!(verbatim.allows(Site::Attribute));
}

#[test]
fn builtin_table_is_unique_and_consistent_with_lookups() {
    let mut names = BTreeSet::new();
    for definition in BUILTIN_ANNOTATIONS {
        assert!(
            names.insert(definition.name),
            "duplicate built-in annotation: {}",
            definition.name
        );
        assert_eq!(
            AnnotationDefinition::lookup(definition.name).map(|entry| entry.name),
            Some(definition.name)
        );
        assert_eq!(
            AnnotationKind::from_name(definition.name),
            Some(definition.kind)
        );
    }
}

#[test]
fn argument_shapes_and_lifecycle_kinds_remain_typed() {
    let upgrade = AnnotationDefinition::lookup("upgrade").expect("upgrade exists");
    assert!(matches!(
        upgrade.arguments,
        ArgumentShape::Named { required, optional }
            if required == ["protocol"]
                && optional.contains(&"heartbeat")
    ));
    assert!(AnnotationKind::Version.is_lifecycle());
    assert!(AnnotationKind::Added.is_lifecycle());
    assert!(AnnotationKind::Removed.is_lifecycle());
    assert!(!AnnotationKind::Deprecated.is_lifecycle());
}
