use super::*;
use crate::versioning::{
    API_VERSION_PROPERTY, API_VERSIONS_PROPERTY, VersionSelection, VersionTimeline, VersioningError,
};

fn timeline() -> VersionTimeline {
    VersionTimeline::from_members(
        "Versions".to_string(),
        &[
            member("v1", Some("1.0")),
            member("v2", Some("2.0")),
            member("v3", None),
        ],
    )
    .expect("timeline")
}

#[test]
fn values_default_to_the_member_ident() {
    let values = timeline()
        .iter()
        .map(|(_, def)| (def.ident.clone(), def.value.clone()))
        .collect::<Vec<_>>();

    assert_eq!(
        values,
        vec![
            ("v1".to_string(), "1.0".to_string()),
            ("v2".to_string(), "2.0".to_string()),
            ("v3".to_string(), "v3".to_string()),
        ]
    );
}

#[test]
fn members_are_found_by_ident_and_value() {
    let timeline = timeline();
    let by_ident = timeline.find("v2").expect("ident");
    let by_value = timeline.find("2.0").expect("value");

    assert_eq!(by_ident, by_value);
    assert_eq!(by_ident.index(), 1);
    assert_eq!(timeline.declared(), "v1 (1.0), v2 (2.0), v3 (v3)");
}

#[test]
fn unknown_and_ambiguous_selections_are_rejected() {
    let timeline = timeline();
    assert!(matches!(
        timeline.find("9.9"),
        Err(VersioningError::UnknownSelectedVersion { .. })
    ));

    let ambiguous = VersionTimeline::from_members(
        "Versions".to_string(),
        &[member("v1", Some("v2")), member("v2", Some("v3"))],
    )
    .expect("timeline");
    assert!(matches!(
        ambiguous.find("v2"),
        Err(VersioningError::AmbiguousSelectedVersion { .. })
    ));
}

#[test]
fn duplicate_values_and_empty_enums_are_rejected() {
    assert!(matches!(
        VersionTimeline::from_members(
            "Versions".to_string(),
            &[member("v1", Some("2.0")), member("v2", Some("2.0"))]
        ),
        Err(VersioningError::DuplicateVersionValue { .. })
    ));
    assert!(matches!(
        VersionTimeline::from_members("Versions".to_string(), &[]),
        Err(VersioningError::EmptyVersionEnum { .. })
    ));
}

#[test]
fn scoped_references_resolve_to_declared_members() {
    let timeline = timeline();

    assert_eq!(
        timeline
            .resolve(&reference("Versions", "v3"))
            .map(|id| id.index()),
        Some(2)
    );
    assert!(timeline.resolve(&reference("Other", "v3")).is_none());
    assert!(timeline.resolve(&reference("Versions", "v9")).is_none());
    assert!(
        timeline
            .resolve(&ScopedName {
                name: vec!["v1".to_string()],
                is_root: false,
            })
            .is_none()
    );
}

#[test]
fn selection_reads_both_properties() {
    let single = VersionSelection::from_properties(&properties(&[(API_VERSION_PROPERTY, "v2")]))
        .expect("selection")
        .expect("present");
    assert_eq!(single, VersionSelection::Versions(vec!["v2".to_string()]));

    let many = VersionSelection::from_properties(&properties(&[(API_VERSIONS_PROPERTY, "v1, v3")]))
        .expect("selection")
        .expect("present");
    assert_eq!(
        many,
        VersionSelection::Versions(vec!["v1".to_string(), "v3".to_string()])
    );

    let all = VersionSelection::from_properties(&properties(&[(API_VERSIONS_PROPERTY, "*")]))
        .expect("selection")
        .expect("present");
    assert_eq!(all, VersionSelection::All);

    let none = VersionSelection::from_properties(&properties(&[])).expect("selection");
    assert!(none.is_none());
}

#[test]
fn selection_rejects_conflicts_and_empty_requests() {
    assert!(matches!(
        VersionSelection::from_properties(&properties(&[
            (API_VERSION_PROPERTY, "v1"),
            (API_VERSIONS_PROPERTY, "v2"),
        ])),
        Err(crate::error::ParseError::Versioning(
            VersioningError::ConflictingSelection { .. }
        ))
    ));
    assert!(matches!(
        VersionSelection::from_properties(&properties(&[(API_VERSIONS_PROPERTY, " , ")])),
        Err(crate::error::ParseError::Versioning(
            VersioningError::EmptySelection
        ))
    ));
}

#[test]
fn selection_resolves_and_sorts_by_declaration_order() {
    let timeline = timeline();
    let selection = VersionSelection::Versions(vec!["v3".to_string(), "1.0".to_string()]);
    let resolved = selection.resolve(&timeline).expect("resolve");

    assert_eq!(
        resolved.iter().map(|id| id.index()).collect::<Vec<_>>(),
        vec![0, 2]
    );
    assert!(matches!(
        VersionSelection::Versions(vec!["9.9".to_string()]).resolve(&timeline),
        Err(crate::error::ParseError::Versioning(
            VersioningError::UnknownSelectedVersion { .. }
        ))
    ));
}

#[test]
fn nested_version_enums_resolve_by_ident_or_path() {
    let timeline = VersionTimeline::from_members(
        "core::ApiVersions".to_string(),
        &[member("v1", None), member("v2", None)],
    )
    .expect("timeline");

    assert_eq!(timeline.enum_ident(), "ApiVersions");
    assert_eq!(
        timeline
            .resolve(&reference("ApiVersions", "v2"))
            .map(|id| id.index()),
        Some(1)
    );
    assert_eq!(
        timeline
            .resolve(&ScopedName {
                name: vec![
                    "core".to_string(),
                    "ApiVersions".to_string(),
                    "v2".to_string()
                ],
                is_root: false,
            })
            .map(|id| id.index()),
        Some(1)
    );
    assert!(timeline.resolve(&reference("Other", "v2")).is_none());
}
