use super::*;
use crate::error::ParseError;
use crate::versioning::VersioningError;

fn header_error(text: &str) -> VersioningError {
    match lower_error(text) {
        ParseError::Versioning(error) => error,
        other => panic!("expected a versioning error, got {other:?}"),
    }
}

#[test]
fn the_version_header_is_recorded_on_the_record() {
    let (_, record) = analyze(
        r#"
@version(header = "X-Api-Version")
enum Versions {
    @rename("1.0") v1,
    v2,
};

struct Pet {
    string id;
};
"#,
    );

    assert_eq!(record.header(), Some("X-Api-Version"));
}

#[test]
fn a_plain_version_marker_declares_no_header() {
    let (_, record) = analyze(
        r#"
@version
enum Versions {
    v1,
};
"#,
    );

    assert_eq!(record.header(), None);
}

#[test]
fn non_token_header_names_are_rejected() {
    for name in ["Api Version", "api(version)", " Api-Version", ""] {
        let text = format!(
            r#"
@version(header = "{name}")
enum Versions {{
    v1,
}};
"#
        );
        let error = header_error(&text);
        assert!(
            matches!(error, VersioningError::MalformedVersionHeader { .. }),
            "'{name}' should be a malformed header, got {error:?}"
        );
    }
}

#[test]
fn the_header_cannot_be_declared_twice_on_the_enum() {
    let error = header_error(
        r#"
@version(header = "X-Api-Version")
@version(header = "X-Other-Version")
enum Versions {
    v1,
};
"#,
    );

    assert!(matches!(
        error,
        VersioningError::DuplicateVersionHeader { .. }
    ));
}

#[test]
fn header_arguments_take_the_marker_outside_enums() {
    let error = header_error(
        r#"
@version(header = "X-Api-Version")
struct Pet {
    string id;
};
"#,
    );

    match error {
        VersioningError::VersionOutsideEnum { target } => {
            assert!(target.contains("Pet"), "target should name Pet: {target}");
        }
        other => panic!("expected VersionOutsideEnum, got {other:?}"),
    }
}

#[test]
fn unsupported_version_arguments_stay_rejected() {
    let error = header_error(
        r#"
@version(query = "api-version")
enum Versions {
    v1,
};
"#,
    );

    assert!(matches!(
        error,
        VersioningError::MalformedVersionMarker { .. }
    ));
}
