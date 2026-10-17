use super::*;
use crate::error::ParseError;
use crate::hir::{HttpParameterLocation, VersionParameter};
use crate::versioning::VersioningError;

#[test]
fn version_parameter_tracks_location_name_and_renamed_values() {
    for (spelling, location) in [
        ("HeAdEr", HttpParameterLocation::Header),
        ("QUERY", HttpParameterLocation::Query),
        ("cookie", HttpParameterLocation::Cookie),
    ] {
        let (_, record) = analyze(&format!(
            r#"
            @version(in = "{spelling}", name = "X-Version")
            enum Versions {{ @rename("") empty, @rename("2026-07") v2 }};
            module Business {{ struct Item {{ string id; }}; }};
        "#
        ));
        assert_eq!(
            record.parameter(),
            Some(&VersionParameter {
                location,
                name: "X-Version".to_string(),
            })
        );
        let values: Vec<_> = record
            .timeline()
            .iter()
            .map(|(_, version)| version.value.as_str())
            .collect();
        assert_eq!(values, ["", "2026-07"]);
    }
    let (_, bare) = analyze("@version enum Versions { v1 }; ");
    assert_eq!(bare.parameter(), None);
}

#[test]
fn version_arguments_reject_missing_unknown_duplicate_and_invalid_values() {
    for arguments in [
        r#"header="X-Version""#,
        r#"in="body", name="version""#,
        r#"in="path", name="version""#,
        r#"in="header""#,
        r#"name="version""#,
        r#"in="header", in="query""#,
        r#"in="header", name="version", extra="x""#,
        r#"in=3, name="version""#,
        r#"in="header", name=3"#,
        r#"in="header", name="Api Version""#,
        r#"in="header", name="""#,
        r#"in="cookie", name="invalid;cookie""#,
        r#"in="query", name="""#,
    ] {
        let error = lower_error(&format!("@version({arguments}) enum Versions {{ v1 }};"));
        assert!(
            matches!(
                error,
                ParseError::Versioning(VersioningError::MalformedVersionParameter { .. })
            ),
            "{arguments}: {error}"
        );
    }
}

#[test]
fn duplicate_version_declarations_and_non_enum_targets_fail() {
    for markers in [
        r#"@version @version(in="header", name="X-Version")"#,
        r#"@version(in="header", name="X-Version") @version"#,
        r#"@version(in="header", name="X-Version") @version(in="query", name="version")"#,
    ] {
        let error = lower_error(&format!("{markers} enum Versions {{ v1 }};"));
        assert!(
            matches!(
                error,
                ParseError::Versioning(VersioningError::DuplicateVersionParameter { .. })
            ),
            "{error}"
        );
    }
    let error = lower_error(r#"@version(in="query", name="version") struct Item { string id; };"#);
    assert!(
        matches!(
            error,
            ParseError::Versioning(VersioningError::VersionOutsideEnum { .. })
        ),
        "{error}"
    );
}

#[test]
fn header_versions_must_be_representable_on_the_wire() {
    for value in ["版本", " leading", "trailing ", "\tleading", "trailing\t"] {
        let source = format!(
            r#"@version(in="header", name="X-Version") enum Versions {{ @rename("{value}") v1 }};"#
        );
        let error = lower_error(&source);
        assert!(
            matches!(
                error,
                ParseError::Versioning(VersioningError::MalformedVersionParameter { .. })
            ),
            "{value:?}: {error}"
        );
    }
    for location in ["query", "cookie"] {
        analyze(&format!(
            r#"@version(in="{location}", name="version") enum Versions {{ @rename("版本 +;/%") v1 }};"#
        ));
    }
}

#[test]
fn version_headers_cannot_replace_http_transport_metadata() {
    for name in [
        "Accept",
        "Content-Type",
        "Content-Length",
        "Content-Encoding",
        "Content-Range",
        "Transfer-Encoding",
        "Host",
        "Connection",
        "Upgrade",
        "Trailer",
        "TE",
        "Cookie",
        "Set-Cookie",
        "Sec-WebSocket-Key",
        "Sec-WebSocket-Protocol",
    ] {
        let source = format!(r#"@version(in="header", name="{name}") enum Versions {{ v1 }};"#);
        let error = lower_error(&source);
        assert!(
            error
                .to_string()
                .contains("reserved for HTTP transport metadata"),
            "{name}: {error}"
        );
    }
}
