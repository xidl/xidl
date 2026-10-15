use crate::hir::{
    Annotation, ConstrTypeDcl, Declarator, Definition, Enumerator, ModuleDcl, ParserProperties,
    Pragma, ScopedName, Specification, TypeDcl,
};
use serde_json::Value;

mod analyze;
mod carriers;
mod error;
mod lifecycle;
mod project;
mod references;
mod selector;
mod spine;
mod timeline;

/// Lowers IDL text into source HIR without interface expansion.
pub(super) fn parse(text: &str) -> Specification {
    let typed = crate::parser::parser_text(text).expect("IDL should parse");
    Specification::from_typed_ast_with_properties_and_path(typed, test_properties(), "test.idl")
        .expect("IDL should lower")
}

/// Lowers IDL text, expecting a failure.
pub(super) fn lower_error(text: &str) -> crate::error::ParseError {
    let typed = crate::parser::parser_text(text).expect("IDL should parse");
    match Specification::from_typed_ast_with_properties_and_path(
        typed,
        test_properties(),
        "test.idl",
    ) {
        Err(error) => error,
        Ok(_) => panic!("expected lowering to fail"),
    }
}

/// Properties that keep the lowering focused on versioning.
pub(super) fn test_properties() -> ParserProperties {
    ParserProperties::from([("expand_interface".to_string(), Value::Bool(false))])
}

/// Analyses IDL text that must declare a version timeline.
pub(super) fn analyze(text: &str) -> (Specification, crate::versioning::VersioningRecord) {
    let spec = parse(text);
    let record = crate::versioning::VersioningRecord::analyze(&spec)
        .expect("analysis should succeed")
        .expect("specification should be versioned");
    (spec, record)
}

/// Builds an enum member with an optional `@rename` display value.
pub(super) fn member(ident: &str, value: Option<&str>) -> Enumerator {
    Enumerator {
        annotations: value
            .map(|value| {
                vec![Annotation::Rename {
                    name: value.to_string(),
                }]
            })
            .unwrap_or_default(),
        ident: ident.to_string(),
    }
}

/// Builds an `Enum::member` version reference.
pub(super) fn reference(enum_ident: &str, member: &str) -> ScopedName {
    ScopedName {
        name: vec![enum_ident.to_string(), member.to_string()],
        is_root: false,
    }
}

/// Builds `@added(Enum::member)`.
pub(super) fn added(enum_ident: &str, member: &str) -> Annotation {
    Annotation::Added {
        version: reference(enum_ident, member),
    }
}

/// Builds `@removed(Enum::member)`.
pub(super) fn removed(enum_ident: &str, member: &str) -> Annotation {
    Annotation::Removed {
        version: reference(enum_ident, member),
    }
}

/// Builds generation properties from string pairs.
pub(super) fn properties(pairs: &[(&str, &str)]) -> ParserProperties {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_string(), Value::String((*value).to_string())))
        .collect()
}

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

fn raised_exceptions(spec: &Specification) -> Vec<String> {
    projected_module(spec)
        .definition
        .iter()
        .find_map(|definition| match definition {
            Definition::InterfaceDcl(interface) => Some(interface),
            _ => None,
        })
        .map(|interface| match &interface.decl {
            crate::hir::InterfaceDclInner::InterfaceDef(def) => def
                .interface_body
                .as_ref()
                .map(|body| {
                    body.0
                        .iter()
                        .filter_map(|export| match export {
                            crate::hir::Export::OpDcl(operation) => Some(operation),
                            _ => None,
                        })
                        .flat_map(|operation| {
                            operation
                                .raises
                                .as_ref()
                                .map(|raises| {
                                    raises
                                        .0
                                        .iter()
                                        .map(|name| name.name.join("::"))
                                        .collect::<Vec<_>>()
                                })
                                .unwrap_or_default()
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
            crate::hir::InterfaceDclInner::InterfaceForwardDcl(_) => Vec::new(),
        })
        .unwrap_or_default()
}
