use super::*;
use crate::error::ParseError;
use crate::versioning::VersioningError;

const LINKED: &str = r#"
@version enum Versions { v1, v2, v3 };
struct Message {
    string id;
    @removed(Versions::v3) string text;
    @added(Versions::v3) @replaces(text) string body;
    @removed(Versions::v3) @optional string tag;
    @added(Versions::v2) @replaces(tag) @optional string label;
    @removed(Versions::v3) @optional string subject;
};
"#;

const UNLINKED: &str = r#"
@version enum Versions { v1, v2, v3 };
struct Message {
    string id;
    @removed(Versions::v3) string text;
    @added(Versions::v3) string body;
    @removed(Versions::v3) @optional string tag;
    @added(Versions::v2) @optional string label;
    @removed(Versions::v3) @optional string subject;
};
"#;

fn versioning_error(text: &str) -> VersioningError {
    match lower_error(text) {
        ParseError::Versioning(error) => error,
        other => panic!("expected a versioning error, got {other:?}"),
    }
}

fn invalid_reason(text: &str) -> String {
    match versioning_error(text) {
        VersioningError::InvalidReplaces { decl, reason } => {
            assert!(decl.starts_with("Message::"), "{reason}");
            reason
        }
        other => panic!("expected an invalid replaces link, got {other}"),
    }
}

fn map_json(text: &str) -> Value {
    let (spec, record) = analyze(text);
    let map = crate::versioning::map::from_spec(&spec, &record, "replaces.idl").expect("map");
    serde_json::to_value(map).expect("serializable map")
}

fn step_members(map: &Value, step: usize) -> &Value {
    &map["steps"][step]["types"]["Message"]["members"]
}

#[test]
fn replaces_keeps_the_written_intervals_and_the_link() {
    let (linked, linked_record) = analyze(LINKED);
    let (unlinked, unlinked_record) = analyze(UNLINKED);
    assert_eq!(
        linked_record.member_intervals(&linked).expect("intervals"),
        unlinked_record
            .member_intervals(&unlinked)
            .expect("intervals")
    );
    for version in project_ids(&linked_record) {
        let linked = linked_record.project(&linked, version).expect("projection");
        let unlinked = unlinked_record
            .project(&unlinked, version)
            .expect("projection");
        assert_eq!(
            serde_json::to_value(&linked).expect("hir"),
            serde_json::to_value(&unlinked).expect("hir"),
            "version {}",
            version.index()
        );
    }
    assert_eq!(
        struct_members(
            &linked_record
                .project(&linked, project_ids(&linked_record)[2])
                .expect("v3"),
            "Message"
        ),
        vec!["id", "body", "label"]
    );
}

#[test]
fn the_version_map_links_the_added_member_to_its_predecessor() {
    let map = map_json(LINKED);
    let members = step_members(&map, 1);
    assert_eq!(members["body"]["change"], "added");
    assert_eq!(members["body"]["to"]["replaces"], "text");
    assert_eq!(members["text"]["change"], "removed");
    assert_eq!(members["tag"]["change"], "removed");
    assert_eq!(members["subject"]["change"], "removed");
    assert!(members["subject"].get("replaces").is_none());
    // The link rides on the successor from its first version onward, so the
    // introduction step already copies the replaced value mechanically.
    let first = step_members(&map, 0);
    assert_eq!(first["label"]["change"], "added");
    assert_eq!(first["label"]["to"]["replaces"], "tag");
}

#[test]
fn replaces_requires_a_version_timeline() {
    let error = versioning_error("struct Message { @removed(Versions::v3) string text; };");
    assert!(matches!(
        error,
        VersioningError::MissingVersionEnum { ref annotation, .. }
            if annotation == "@removed"
    ));
}

#[test]
fn replaces_names_an_existing_sibling_retired_at_one_step() {
    let reason = invalid_reason(
        r#"
        @version enum Versions { v1, v2 };
        struct Message {
            @added(Versions::v2) @replaces(missing) string body;
        };
        "#,
    );
    assert!(reason.contains("names missing, but no member"), "{reason}");

    let reason = invalid_reason(
        r#"
        @version enum Versions { v1, v2 };
        struct Message {
            string text;
            @added(Versions::v2) @replaces(text) string body;
        };
        "#,
    );
    assert!(
        reason.contains("names text, which is not removed"),
        "{reason}"
    );

    let reason = invalid_reason(
        r#"
        @version enum Versions { v1, v2, v3 };
        struct Message {
            @removed(Versions::v2) string text;
            @added(Versions::v3) @replaces(text) string body;
        };
        "#,
    );
    assert!(reason.contains("after text retires at"), "{reason}");

    let reason = invalid_reason(
        r#"
        @version enum Versions { v1, v2, v3 };
        struct Message {
            @removed(Versions::v3) string text;
            @added(Versions::v1) @removed(Versions::v2) @replaces(text) string body;
        };
        "#,
    );
    assert!(reason.contains("itself retires at"), "{reason}");
}

#[test]
fn replaces_rejects_self_links_and_ambiguous_targets() {
    let reason = invalid_reason(
        r#"
        @version enum Versions { v1, v2 };
        struct Message {
            @removed(Versions::v2) @replaces(body) string body;
        };
        "#,
    );
    assert!(reason.contains("replaces itself"), "{reason}");

    let reason = invalid_reason(
        r#"
        @version enum Versions { v1, v2 };
        struct Message {
            @removed(Versions::v2) string text;
            @added(Versions::v2) @replaces(text) string first;
            @added(Versions::v2) @replaces(text) string second;
        };
        "#,
    );
    assert!(reason.contains("names text more than once"), "{reason}");
}

#[test]
fn replaces_requires_a_single_declarator() {
    let reason = invalid_reason(
        r#"
        @version enum Versions { v1, v2 };
        struct Message {
            @removed(Versions::v2) string text;
            @added(Versions::v2) @replaces(text) string a, b;
        };
        "#,
    );
    assert!(reason.contains("single declarator"), "{reason}");
}

#[test]
fn malformed_arguments_and_non_member_sites_are_rejected() {
    // Arguments that do not lower stay raw, so the availability walk names
    // the malformed application before the timeline is built.
    let error = versioning_error(
        r#"
        @version enum Versions { v1, v2 };
        struct Message {
            @removed(Versions::v2) string text;
            @added(Versions::v2) @replaces(Versions::v2, "text") string body;
        };
        "#,
    );
    assert!(
        matches!(error, VersioningError::MalformedReplaces { .. }),
        "{error:?}"
    );

    let error = versioning_error(
        r#"
        @version enum Versions { v1, v2 };
        struct Message {
            @removed(Versions::v2) string text;
        };
        @replaces(text) interface Reader {};
        "#,
    );
    assert!(matches!(
        error,
        VersioningError::UnsupportedVersionTarget { ref annotation, .. }
            if annotation == "@replaces"
    ));
}

#[test]
fn the_link_survives_only_on_the_declaration_that_carries_it() {
    let (linked, record) = analyze(LINKED);
    let projected = record
        .project(&linked, project_ids(&record)[1])
        .expect("v2");
    let module = projected
        .0
        .iter()
        .find_map(|definition| match definition {
            Definition::ModuleDcl(module) => Some(module),
            _ => None,
        })
        .expect("projection module");
    let members = module
        .definition
        .iter()
        .find_map(|definition| match definition {
            Definition::TypeDcl(TypeDcl::ConstrTypeDcl(ConstrTypeDcl::StructDcl(value))) => {
                Some(&value.member)
            }
            _ => None,
        })
        .expect("message");
    for member in members {
        assert!(
            !member
                .annotations
                .iter()
                .any(|annotation| matches!(annotation, Annotation::Replaces { .. })),
            "projection strips the link"
        );
    }
}
