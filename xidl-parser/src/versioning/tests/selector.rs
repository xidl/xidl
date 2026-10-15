use crate::hir::{Definition, Pragma, VersionSelectorLocation};

use super::analyze;

const TIMELINE: &str = "
@version
enum ApiVersions {
    @rename(\"2025-01\") v1,
    @rename(\"2025-06\") v2,
};
";

fn selector_text(selector: &str) -> String {
    format!(
        "@version\n{selector}\nenum ApiVersions {{\n    @rename(\"2025-01\") v1,\n    @rename(\"2025-06\") v2,\n}};\nstruct Pet {{ string id; }};\n"
    )
}

#[test]
fn header_selector_is_recorded_on_the_timeline() {
    let (_, record) = analyze(&selector_text(
        "@version_selector(header = \"Api-Version\")",
    ));
    let selector = record.selector().expect("selector declared");
    assert_eq!(selector.location, VersionSelectorLocation::Header);
    assert_eq!(selector.name, "Api-Version");
}

#[test]
fn query_selector_is_recorded_on_the_timeline() {
    let (_, record) = analyze(&selector_text("@version_selector(query = \"api-version\")"));
    let selector = record.selector().expect("selector declared");
    assert_eq!(selector.location, VersionSelectorLocation::Query);
    assert_eq!(selector.name, "api-version");
}

#[test]
fn unversioned_contracts_have_no_selector() {
    let (_, record) = analyze(TIMELINE);
    assert!(record.selector().is_none());
}

#[test]
fn both_locations_are_malformed() {
    let error = super::lower_error(&selector_text(
        "@version_selector(header = \"Api-Version\", query = \"api-version\")",
    ));
    assert!(
        error
            .to_string()
            .contains("requires exactly one of header = \"...\" or query = \"...\""),
        "{error}"
    );
}

#[test]
fn empty_selector_name_is_malformed() {
    let error = super::lower_error(&selector_text("@version_selector(header = \"\")"));
    assert!(error.to_string().contains("@version_selector"), "{error}");
}

#[test]
fn positional_argument_is_malformed() {
    let error = super::lower_error(&selector_text("@version_selector(\"Api-Version\")"));
    assert!(
        error
            .to_string()
            .contains("requires exactly one of header = \"...\" or query = \"...\""),
        "{error}"
    );
}

#[test]
fn selector_outside_the_version_enum_is_rejected() {
    let error = super::lower_error(&format!(
        "{TIMELINE}\n@version_selector(header = \"Api-Version\")\nstruct Pet {{ string id; }};\n"
    ));
    assert!(
        error.to_string().contains("must sit on the @version enum"),
        "{error}"
    );
}

#[test]
fn selector_without_version_marker_is_rejected() {
    let error = super::lower_error(
        "@version_selector(header = \"Api-Version\")\nenum ApiVersions { v1, v2 };\n",
    );
    assert!(
        error.to_string().contains("must sit on the @version enum"),
        "{error}"
    );
}

#[test]
fn two_selectors_on_one_enum_are_rejected() {
    let error = super::lower_error(&selector_text(
        "@version_selector(header = \"Api-Version\")\n@version_selector(query = \"api-version\")",
    ));
    assert!(
        error.to_string().contains("multiple @version_selector"),
        "{error}"
    );
}

#[test]
fn projection_synthesizes_the_wire_identity_pragma() {
    let (spec, record) = analyze(&selector_text(
        "@version_selector(header = \"Api-Version\")",
    ));
    let projected = record
        .project(
            &spec,
            record
                .timeline()
                .iter()
                .map(|(id, _)| id)
                .next()
                .expect("v1"),
        )
        .expect("projection should succeed");
    let mut selector_pragma = None;
    let mut versions_pragma = false;
    for definition in &projected.0 {
        if let Definition::Pragma(Pragma::XidlcApiVersions { selector, versions }) = definition {
            selector_pragma = selector
                .as_ref()
                .map(|selector| (selector.location.clone(), selector.name.clone()));
            versions_pragma = versions.len() == 2;
        }
    }
    assert_eq!(
        selector_pragma,
        Some(("header".to_string(), "Api-Version".to_string()))
    );
    assert!(versions_pragma, "both versions must travel in the pragma");
}

#[test]
fn merged_projection_keeps_one_wire_identity_pragma() {
    let (spec, record) = analyze(&selector_text("@version_selector(query = \"api-version\")"));
    let merged = record
        .merge(&spec, &project_ids(&record))
        .expect("merge should succeed");
    let count = merged
        .0
        .iter()
        .filter(|definition| {
            matches!(
                definition,
                Definition::Pragma(Pragma::XidlcApiVersions { .. })
            )
        })
        .count();
    assert_eq!(count, 1, "the merged projection keeps one identity pragma");
}

fn project_ids(record: &crate::versioning::VersioningRecord) -> Vec<crate::versioning::VersionId> {
    record.timeline().iter().map(|(id, _)| id).collect()
}
