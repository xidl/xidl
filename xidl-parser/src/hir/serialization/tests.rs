use super::*;
use crate::hir::{CustomPragma, Pragma};

fn builtin(name: &str) -> Annotation {
    match name {
        "final" => Annotation::Final,
        "appendable" => Annotation::Appendable,
        "mutable" => Annotation::Mutable,
        other => Annotation::Builtin {
            name: other.to_string(),
            params: None,
        },
    }
}

fn raw_extensibility(value: &str) -> Annotation {
    Annotation::Builtin {
        name: "extensibility".to_string(),
        params: Some(AnnotationParams::Raw(format!("\"{value}\""))),
    }
}

#[test]
fn apply_pragma_updates_config_and_ignores_non_serialization_pragmas() {
    let mut config = SerializeConfig {
        explicit_kind: Some(SerializeKind::PlCdr),
        version: None,
    };

    config.apply_pragma(Pragma::XidlcPackage("demo.pkg".to_string()));
    config.apply_pragma(Pragma::XidlcOpenApiVersion("3.1.0".to_string()));
    config.apply_pragma(Pragma::XidlcOpenApiService {
        base_url: "https://demo.test".to_string(),
        description: None,
    });
    config.apply_pragma(Pragma::Custom(CustomPragma {
        directive: "#pragma".to_string(),
        argument: Some("vendor keep-me".to_string()),
    }));
    assert!(matches!(config.explicit_kind, Some(SerializeKind::PlCdr)));

    config.apply_pragma(Pragma::XidlcVersion(SerializeVersion::Xcdr2));
    assert!(config.explicit_kind.is_none());
    assert!(matches!(config.version, Some(SerializeVersion::Xcdr2)));

    config.apply_pragma(Pragma::XidlcSerialize(SerializeKind::DelimitedCdr));
    assert!(matches!(
        config.resolve(Extensibility::Appendable),
        SerializeKind::DelimitedCdr
    ));
}

#[test]
fn resolves_versions_and_annotation_extensibility() {
    let mut config = SerializeConfig::default();
    assert!(matches!(
        config.resolve(Extensibility::None),
        SerializeKind::Cdr
    ));

    config.version = Some(SerializeVersion::Xcdr1);
    assert!(matches!(
        config.resolve(Extensibility::Final),
        SerializeKind::Cdr
    ));
    assert!(matches!(
        config.resolve(Extensibility::Appendable),
        SerializeKind::Cdr
    ));
    assert!(matches!(
        config.resolve(Extensibility::Mutable),
        SerializeKind::PlCdr
    ));
    assert!(matches!(
        config.resolve(Extensibility::None),
        SerializeKind::PlainCdr
    ));

    config.version = Some(SerializeVersion::Xcdr2);
    assert!(matches!(
        config.resolve(Extensibility::Final),
        SerializeKind::PlainCdr2
    ));
    assert!(matches!(
        config.resolve(Extensibility::Appendable),
        SerializeKind::DelimitedCdr
    ));
    assert!(matches!(
        config.resolve(Extensibility::Mutable),
        SerializeKind::PlCdr2
    ));
    assert!(matches!(
        config.resolve(Extensibility::None),
        SerializeKind::Cdr
    ));
}

#[test]
fn derives_extensibility_from_builtin_and_raw_annotations() {
    assert!(matches!(
        extensibility_from_annotations(&[builtin("final")]),
        Extensibility::Final
    ));
    assert!(matches!(
        extensibility_from_annotations(&[builtin("appendable")]),
        Extensibility::Appendable
    ));
    assert!(matches!(
        extensibility_from_annotations(&[builtin("mutable")]),
        Extensibility::Mutable
    ));
    assert!(matches!(
        extensibility_from_annotations(&[raw_extensibility("final")]),
        Extensibility::Final
    ));
    assert!(matches!(
        extensibility_from_annotations(&[raw_extensibility("appendable")]),
        Extensibility::Appendable
    ));
    assert!(matches!(
        extensibility_from_annotations(&[raw_extensibility("mutable")]),
        Extensibility::Mutable
    ));
    assert!(matches!(
        extensibility_from_annotations(&[builtin("unknown")]),
        Extensibility::None
    ));
}
