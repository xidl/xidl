use crate::versioning::VersioningError;

#[test]
fn every_diagnostic_names_the_failing_contract() {
    let cases = vec![
        (
            VersioningError::VersionOutsideEnum {
                target: "Pet".to_string(),
            },
            "@version is only valid on an enum declaration",
        ),
        (
            VersioningError::DuplicateVersionEnum {
                first: "Versions".to_string(),
                second: "Other".to_string(),
            },
            "multiple @version enums declared",
        ),
        (
            VersioningError::EmptyVersionEnum {
                ident: "Versions".to_string(),
            },
            "declares no versions",
        ),
        (
            VersioningError::DuplicateVersionValue {
                value: "2.0".to_string(),
                first: "v1".to_string(),
                second: "v2".to_string(),
            },
            "both resolve to value",
        ),
        (
            VersioningError::MissingVersionEnum {
                annotation: "@added".to_string(),
                decl: "Pet.age".to_string(),
            },
            "requires a @version enum",
        ),
        (
            VersioningError::MalformedVersionReference {
                annotation: "@added".to_string(),
                decl: "Pet.age".to_string(),
            },
            "requires a version reference",
        ),
        (
            VersioningError::MalformedVersionMarker {
                decl: "Versions".to_string(),
            },
            "takes no arguments",
        ),
        (
            VersioningError::UnknownVersionReference {
                annotation: "@added".to_string(),
                decl: "Pet.age".to_string(),
                reference: "Versions::v9".to_string(),
                declared: "v1 (v1)".to_string(),
            },
            "is not declared",
        ),
        (
            VersioningError::UnsupportedVersionAnnotation {
                name: "@renamedFrom".to_string(),
                decl: "Pet.name".to_string(),
            },
            "is not supported yet",
        ),
        (
            VersioningError::UnsupportedVersionTarget {
                annotation: "@added".to_string(),
                target: "LIMIT (declaration)".to_string(),
            },
            "cannot be applied to",
        ),
        (
            VersioningError::VersionedVersionEnum {
                annotation: "@added".to_string(),
                ident: "Versions::v1".to_string(),
            },
            "cannot carry",
        ),
        (
            VersioningError::DuplicateLifecycleAnnotation {
                annotation: "@added".to_string(),
                decl: "Pet.age".to_string(),
            },
            "duplicate @added",
        ),
        (
            VersioningError::EmptyAvailability {
                decl: "Pet.age".to_string(),
                added: "Versions::v2".to_string(),
                removed: "Versions::v2".to_string(),
            },
            "unavailable in every version",
        ),
        (
            VersioningError::ChildOutsideParent {
                decl: "Pet.age".to_string(),
                parent: "Pet".to_string(),
                version: "Versions::v1".to_string(),
            },
            "outside the availability of",
        ),
        (
            VersioningError::DanglingReference {
                decl: "Pet.old".to_string(),
                reference: "Old".to_string(),
                version: "v2".to_string(),
            },
            "does not exist in v2",
        ),
        (
            VersioningError::UnversionedSpecification {
                selection: "v1".to_string(),
            },
            "requires a @version enum",
        ),
        (
            VersioningError::UnknownSelectedVersion {
                selection: "9.9".to_string(),
                declared: "v1 (1.0)".to_string(),
            },
            "is not declared",
        ),
        (
            VersioningError::AmbiguousSelectedVersion {
                selection: "v2".to_string(),
                first: "v1".to_string(),
                second: "v2".to_string(),
            },
            "matches both",
        ),
        (
            VersioningError::UnknownVersion { index: 7 },
            "not part of the declared timeline",
        ),
        (
            VersioningError::ConflictingSelection {
                first: "api_version".to_string(),
                second: "api_versions".to_string(),
            },
            "cannot both be set",
        ),
        (VersioningError::EmptySelection, "selection is empty"),
        (
            VersioningError::UnknownVersionLayout {
                value: "sideways".to_string(),
            },
            "unknown version layout",
        ),
    ];

    for (error, needle) in cases {
        let rendered = error.to_string();
        assert!(
            rendered.contains(needle),
            "`{rendered}` should contain `{needle}`"
        );
        assert!(std::error::Error::source(&error).is_none());
    }
}
