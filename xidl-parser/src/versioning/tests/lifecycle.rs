use super::*;
use crate::error::{ParseError, ParserResult};
use crate::versioning::lifecycle::{Availability, Site, SiteKind, availability};
use crate::versioning::{VersionTimeline, VersioningError};

fn timeline() -> VersionTimeline {
    VersionTimeline::from_members(
        "Versions".to_string(),
        &[member("v1", None), member("v2", None), member("v3", None)],
    )
    .expect("timeline")
}

fn site<'a>(path: &str, kind: SiteKind, annotations: &'a [Annotation]) -> Site<'a> {
    Site {
        path: path.to_string(),
        parent_path: "Pet".to_string(),
        kind,
        annotations,
        enum_members: None,
    }
}

fn expect_error(result: ParserResult<Availability>) -> VersioningError {
    match result {
        Err(ParseError::Versioning(error)) => error,
        other => panic!("expected a versioning error, got {other:?}"),
    }
}

#[test]
fn annotations_are_order_independent() {
    let timeline = timeline();
    let ordered = vec![added("Versions", "v2"), removed("Versions", "v3")];
    let reversed = vec![removed("Versions", "v3"), added("Versions", "v2")];

    let first = availability(
        &site("Pet.age", SiteKind::Member, &ordered),
        Availability::full(),
        &timeline,
    )
    .expect("availability");
    let second = availability(
        &site("Pet.age", SiteKind::Member, &reversed),
        Availability::full(),
        &timeline,
    )
    .expect("availability");

    assert_eq!(first, second);
    assert_eq!(first, Availability { from: 1, until: 2 });
}

#[test]
fn lifecycle_annotations_inherit_the_parent_interval() {
    let timeline = timeline();
    let parent = availability(
        &site("Pet", SiteKind::Type, &[added("Versions", "v2")]),
        Availability::full(),
        &timeline,
    )
    .expect("parent availability");
    let child = availability(&site("Pet.age", SiteKind::Member, &[]), parent, &timeline)
        .expect("child availability");

    assert_eq!(child, Availability { from: 1, until: 3 });
}

#[test]
fn duplicate_lifecycle_annotation_is_rejected() {
    let error = expect_error(availability(
        &site(
            "Pet.age",
            SiteKind::Member,
            &[added("Versions", "v2"), added("Versions", "v3")],
        ),
        Availability::full(),
        &timeline(),
    ));

    assert!(matches!(
        error,
        VersioningError::DuplicateLifecycleAnnotation { .. }
    ));
}

#[test]
fn empty_interval_is_rejected() {
    let error = expect_error(availability(
        &site(
            "Pet.age",
            SiteKind::Member,
            &[added("Versions", "v2"), removed("Versions", "v2")],
        ),
        Availability::full(),
        &timeline(),
    ));

    assert!(matches!(error, VersioningError::EmptyAvailability { .. }));
}

#[test]
fn child_outside_parent_interval_is_rejected() {
    let timeline = timeline();
    let parent = Availability { from: 1, until: 3 };
    let error = expect_error(availability(
        &site("Pet.age", SiteKind::Member, &[added("Versions", "v1")]),
        parent,
        &timeline,
    ));

    assert!(matches!(error, VersioningError::ChildOutsideParent { .. }));
}

#[test]
fn removed_beyond_parent_interval_is_rejected() {
    let timeline = timeline();
    let parent = Availability { from: 0, until: 2 };
    let error = expect_error(availability(
        &site("Pet.age", SiteKind::Member, &[removed("Versions", "v3")]),
        parent,
        &timeline,
    ));

    assert!(matches!(error, VersioningError::ChildOutsideParent { .. }));
}

#[test]
fn unknown_version_reference_is_rejected() {
    let error = expect_error(availability(
        &site("Pet.age", SiteKind::Member, &[added("Versions", "v9")]),
        Availability::full(),
        &timeline(),
    ));

    assert!(matches!(
        error,
        VersioningError::UnknownVersionReference { .. }
    ));
}

#[test]
fn malformed_lifecycle_annotation_is_rejected() {
    let annotations = vec![Annotation::Builtin {
        name: "added".to_string(),
        params: None,
    }];
    let error = expect_error(availability(
        &site("Pet.age", SiteKind::Member, &annotations),
        Availability::full(),
        &timeline(),
    ));

    assert!(matches!(
        error,
        VersioningError::MalformedVersionReference { .. }
    ));
}

#[test]
fn unsupported_version_annotation_is_rejected() {
    let annotations = vec![Annotation::Builtin {
        name: "renamedFrom".to_string(),
        params: None,
    }];
    let error = expect_error(availability(
        &site("Pet.age", SiteKind::Member, &annotations),
        Availability::full(),
        &timeline(),
    ));

    assert!(matches!(
        error,
        VersioningError::UnsupportedVersionAnnotation { .. }
    ));
}

#[test]
fn unsupported_target_is_rejected() {
    let error = expect_error(availability(
        &site("LIMIT", SiteKind::Unsupported, &[added("Versions", "v2")]),
        Availability::full(),
        &timeline(),
    ));

    assert!(matches!(
        error,
        VersioningError::UnsupportedVersionTarget { .. }
    ));
}

#[test]
fn availability_contains_selected_versions() {
    let timeline = timeline();
    let value = availability(
        &site(
            "Pet.age",
            SiteKind::Member,
            &[added("Versions", "v2"), removed("Versions", "v3")],
        ),
        Availability::full(),
        &timeline,
    )
    .expect("availability");

    let selected = timeline
        .iter()
        .filter(|(id, _)| value.contains(*id))
        .map(|(_, def)| def.ident.clone())
        .collect::<Vec<_>>();
    assert_eq!(selected, vec!["v2"]);
}

#[test]
fn child_interval_disjoint_from_parent_is_rejected() {
    let timeline = timeline();
    let parent_added = Availability { from: 1, until: 3 };
    let error = expect_error(availability(
        &site("Pet.age", SiteKind::Member, &[removed("Versions", "v2")]),
        parent_added,
        &timeline,
    ));
    assert!(matches!(error, VersioningError::ChildOutsideParent { .. }));

    let parent_removed = Availability { from: 0, until: 1 };
    let error = expect_error(availability(
        &site("Pet.age", SiteKind::Member, &[added("Versions", "v2")]),
        parent_removed,
        &timeline,
    ));
    assert!(matches!(error, VersioningError::ChildOutsideParent { .. }));
}
