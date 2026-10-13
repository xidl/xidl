use super::*;
use crate::error::ParseError;
use crate::hir::{ConstrTypeDcl, Declarator, Definition, ModuleDcl, Pragma, TypeDcl, TypeSpec};
use crate::versioning::{self, VersioningError};

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

fn declarator_name(declarator: &Declarator) -> String {
    match declarator {
        Declarator::SimpleDeclarator(simple) => simple.0.clone(),
        Declarator::ArrayDeclarator(array) => array.ident.clone(),
    }
}

fn projected_module(spec: &Specification) -> &ModuleDcl {
    spec.0
        .iter()
        .find_map(|definition| match definition {
            Definition::ModuleDcl(module) => Some(module),
            _ => None,
        })
        .expect("projection should wrap definitions in a module")
}

fn struct_members(spec: &Specification, ident: &str) -> Vec<String> {
    for definition in &projected_module(spec).definition {
        if let Definition::TypeDcl(TypeDcl::ConstrTypeDcl(ConstrTypeDcl::StructDcl(value))) =
            definition
        {
            if value.ident == ident {
                return value
                    .member
                    .iter()
                    .map(|member| declarator_name(&member.ident[0]))
                    .collect();
            }
        }
    }
    Vec::new()
}

fn has_enum(spec: &Specification, ident: &str) -> bool {
    projected_module(spec).definition.iter().any(|definition| {
        matches!(
            definition,
            Definition::TypeDcl(TypeDcl::ConstrTypeDcl(ConstrTypeDcl::EnumDcl(value)))
                if value.ident == ident
        )
    })
}

fn version_pragma(spec: &Specification) -> Option<String> {
    spec.0.iter().find_map(|definition| match definition {
        Definition::Pragma(Pragma::XidlcOpenApiVersion(value)) => Some(value.clone()),
        _ => None,
    })
}

fn project_ids(record: &crate::versioning::VersioningRecord) -> Vec<crate::versioning::VersionId> {
    record.timeline().iter().map(|(id, _)| id).collect()
}

#[test]
fn projection_selects_members_per_version() {
    let (spec, record) = analyze(VERSIONED);
    let mut signatures = Vec::new();
    for version in project_ids(&record) {
        let projected = versioning::project(&spec, &record, version).expect("projection");
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
    let projected = versioning::project(&spec, &record, version).expect("projection");

    assert_eq!(projected_module(&projected).ident, "v1");
    assert_eq!(version_pragma(&projected).as_deref(), Some("1.0"));
}

#[test]
fn projection_removes_the_version_enum_and_lifecycle_annotations() {
    let (spec, record) = analyze(VERSIONED);
    let version = project_ids(&record)[1];
    let projected = versioning::project(&spec, &record, version).expect("projection");
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
    let projected = versioning::project(&spec, &record, version).expect("projection");

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

    versioning::project(&spec, &record, versions[0]).expect("v1 keeps Old");
    match versioning::project(&spec, &record, versions[1]) {
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
fn merge_keeps_one_module_per_version_in_order() {
    let (spec, record) = analyze(VERSIONED);
    let versions = project_ids(&record);
    let merged = versioning::merge(&spec, &record, &versions).expect("merge");

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
    let merged = versioning::merge(&spec, &record, &[versions[2], versions[0]]).expect("merge");

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
    let projected = versioning::project(&spec, &record, versions[1]).expect("projection");

    insta::assert_debug_snapshot!("projection_v2", projected);
}
