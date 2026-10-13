use super::*;
use crate::error::ParseError;
use crate::hir::{ConstrTypeDcl, Definition, TypeDcl, TypeSpec};
use crate::versioning::VersioningError;

const VERSIONED: &str = r#"
#pragma xidlc package versioned

@version
enum Versions {
    @rename("1.0") v1,
    @rename("2.0") v2,
    v3,
};

struct Item {
    string id;
};

struct Pet {
    string id;
    ::Item item;
    @added(Versions::v2)
    @removed(Versions::v3)
    int32 age;
    @added(Versions::v3)
    @rename("age")
    int32 age2_;
};
"#;

#[test]
fn projection_selects_members_per_version() {
    let (spec, record) = analyze(VERSIONED);
    let mut signatures = Vec::new();
    for version in project_ids(&record) {
        let projected = record.project(&spec, version).expect("projection");
        let ident = record
            .timeline()
            .get(version)
            .expect("version definition")
            .ident
            .clone();
        signatures.push((ident, struct_members(&projected, "Pet")));
    }

    assert_eq!(
        signatures,
        vec![
            ("v1".to_string(), vec!["id".to_string(), "item".to_string()]),
            (
                "v2".to_string(),
                vec!["id".to_string(), "item".to_string(), "age".to_string()]
            ),
            (
                "v3".to_string(),
                vec!["id".to_string(), "item".to_string(), "age2_".to_string()]
            ),
        ]
    );
}

#[test]
fn projection_wraps_the_version_and_sets_the_document_version() {
    let (spec, record) = analyze(VERSIONED);
    let version = project_ids(&record)[0];
    let projected = record.project(&spec, version).expect("projection");

    assert_eq!(projected_module(&projected).ident, "v1");
    assert_eq!(version_pragma(&projected).as_deref(), Some("1.0"));
}

#[test]
fn projection_removes_the_version_enum_and_lifecycle_annotations() {
    let (spec, record) = analyze(VERSIONED);
    let version = project_ids(&record)[1];
    let projected = record.project(&spec, version).expect("projection");
    let json = serde_json::to_string(&projected).expect("serialize");

    assert!(!has_enum(&projected, "Versions"));
    assert!(!json.contains("\"Added\""));
    assert!(!json.contains("\"Removed\""));
    assert!(!json.contains("\"Version\""));
}

#[test]
fn projection_makes_root_references_relative() {
    let (spec, record) = analyze(VERSIONED);
    let version = project_ids(&record)[0];
    let projected = record.project(&spec, version).expect("projection");

    let item = projected_module(&projected)
        .definition
        .iter()
        .find_map(|definition| match definition {
            Definition::TypeDcl(TypeDcl::ConstrTypeDcl(ConstrTypeDcl::StructDcl(value)))
                if value.ident == "Pet" =>
            {
                value.member.iter().find_map(|member| match &member.ty {
                    TypeSpec::ScopedName(name) if name.name == ["Item"] => Some(name.clone()),
                    _ => None,
                })
            }
            _ => None,
        })
        .expect("Pet.item should reference Item");

    assert!(!item.is_root);
}

#[test]
fn projection_reports_references_to_removed_declarations() {
    let (spec, record) = analyze(
        r#"
@version
enum Versions { v1, v2 };

@removed(Versions::v2)
struct Old {
    string id;
};

struct Pet {
    Old old;
};
"#,
    );
    let versions = project_ids(&record);

    record.project(&spec, versions[0]).expect("v1 keeps Old");
    match record.project(&spec, versions[1]) {
        Err(ParseError::Versioning(VersioningError::DanglingReference {
            decl,
            reference,
            version,
        })) => {
            assert!(decl.contains("Pet"));
            assert_eq!(reference, "Old");
            assert_eq!(version, "v2");
        }
        other => panic!("expected a dangling reference error, got {other:?}"),
    }
}

#[test]
fn projection_prunes_raises_entries_of_removed_exceptions() {
    let (spec, record) = analyze(
        r#"
@version
enum Versions { v1, v2 };

@http(404)
exception NotFound {
    string id;
};

@added(Versions::v2)
@http(409)
exception Conflict {
    string reason;
};

interface Pets {
    @post(path = "/pets/{id}")
    void adopt(@path string id) raises(NotFound, Conflict);
};
"#,
    );
    let versions = project_ids(&record);
    let v1 = record.project(&spec, versions[0]).expect("v1 projection");
    let v2 = record.project(&spec, versions[1]).expect("v2 projection");

    assert_eq!(raised_exceptions(&v1), vec!["NotFound"]);
    assert_eq!(raised_exceptions(&v2), vec!["NotFound", "Conflict"]);
}

#[test]
fn merge_keeps_one_module_per_version_in_order() {
    let (spec, record) = analyze(VERSIONED);
    let versions = project_ids(&record);
    let merged = record.merge(&spec, &versions).expect("merge");

    let modules = merged
        .0
        .iter()
        .filter_map(|definition| match definition {
            Definition::ModuleDcl(module) => Some(module.ident.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(modules, vec!["v1", "v2", "v3"]);

    let pragmas = merged
        .0
        .iter()
        .filter(|definition| matches!(definition, Definition::Pragma(_)))
        .count();
    assert_eq!(pragmas, 1);
    assert!(version_pragma(&merged).is_none());
}

#[test]
fn merge_respects_the_selected_subset() {
    let (spec, record) = analyze(VERSIONED);
    let versions = project_ids(&record);
    let merged = record
        .merge(&spec, &[versions[2], versions[0]])
        .expect("merge");

    let modules = merged
        .0
        .iter()
        .filter_map(|definition| match definition {
            Definition::ModuleDcl(module) => Some(module.ident.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(modules, vec!["v3", "v1"]);
}

#[test]
fn projection_snapshot_locks_the_desugaring() {
    let (spec, record) = analyze(VERSIONED);
    let versions = project_ids(&record);
    let projected = record.project(&spec, versions[1]).expect("projection");

    insta::assert_debug_snapshot!("projection_v2", projected);
}
