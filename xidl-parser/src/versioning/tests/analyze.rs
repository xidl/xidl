use super::*;
use crate::error::ParseError;
use crate::versioning::{self, VersioningError};

fn analyze_error(text: &str) -> VersioningError {
    match lower_error(text) {
        ParseError::Versioning(error) => error,
        other => panic!("expected a versioning error, got {other:?}"),
    }
}

#[test]
fn version_enum_is_discovered() {
    let (_, record) = analyze(
        r#"
@version
enum Versions {
    @rename("1.0") v1,
    v2,
};

struct Pet {
    string id;
};
"#,
    );

    assert_eq!(record.timeline().enum_ident(), "Versions");
    assert_eq!(record.timeline().len(), 2);
    assert_eq!(record.timeline().declared(), "v1 (1.0), v2 (v2)");
}

#[test]
fn unversioned_specifications_are_accepted() {
    let spec = parse("struct Pet { string id; };");
    assert!(versioning::analyze(&spec).expect("analysis").is_none());
}

#[test]
fn version_marker_outside_an_enum_is_rejected() {
    let error = analyze_error(
        r#"
@version
struct Pet {
    string id;
};
"#,
    );

    assert!(matches!(error, VersioningError::VersionOutsideEnum { .. }));
}

#[test]
fn version_marker_on_a_member_is_rejected() {
    let error = analyze_error(
        r#"
@version
enum Versions { v1, v2 };

struct Pet {
    @version
    string id;
};
"#,
    );

    assert!(matches!(error, VersioningError::VersionOutsideEnum { .. }));
}

#[test]
fn version_marker_with_arguments_is_rejected() {
    let error = analyze_error(
        r#"
@version("1.0")
enum Versions { v1, v2 };
"#,
    );

    assert!(matches!(
        error,
        VersioningError::MalformedVersionMarker { .. }
    ));
}

#[test]
fn duplicate_version_enums_are_rejected() {
    let error = analyze_error(
        r#"
@version
enum Versions { v1 };

@version
enum Other { v1 };
"#,
    );

    assert!(matches!(
        error,
        VersioningError::DuplicateVersionEnum { .. }
    ));
}

#[test]
fn empty_version_enum_is_rejected() {
    let error = analyze_error("@version\nenum Versions { };");
    assert!(matches!(error, VersioningError::EmptyVersionEnum { .. }));
}

#[test]
fn lifecycle_annotation_without_a_version_enum_is_rejected() {
    let error = analyze_error(
        r#"
struct Pet {
    @added(Versions::v2)
    int32 age;
};
"#,
    );

    assert!(matches!(error, VersioningError::MissingVersionEnum { .. }));
}

#[test]
fn lifecycle_annotations_on_the_version_enum_are_rejected() {
    let error = analyze_error(
        r#"
@version
enum Versions {
    @added(Versions::v2)
    v1,
    v2,
};
"#,
    );

    assert!(matches!(
        error,
        VersioningError::VersionedVersionEnum { .. }
    ));
}

#[test]
fn unsupported_version_annotations_are_rejected() {
    let error = analyze_error(
        r#"
@version
enum Versions { v1, v2 };

struct Pet {
    @renamedFrom(Versions::v2, "old")
    string name;
};
"#,
    );

    assert!(matches!(
        error,
        VersioningError::UnsupportedVersionAnnotation { .. }
    ));
}

#[test]
fn malformed_lifecycle_arguments_are_rejected() {
    let error = analyze_error(
        r#"
@version
enum Versions { v1, v2 };

struct Pet {
    @added("2.0")
    int32 age;
};
"#,
    );

    assert!(matches!(
        error,
        VersioningError::MalformedVersionReference { .. }
    ));
}
