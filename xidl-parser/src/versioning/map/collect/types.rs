//! Collects declaration and member surfaces from projected HIR.

use super::super::helpers::descriptor;
use super::super::model::{TypeParts, TypeSurface};
use super::render::{
    declarator_facts, declarator_ident, join, render_bitfield_type, render_element_type,
    render_scoped_name, render_switch_type, render_type,
};
use crate::hir::{
    self, Definition, Specification, TypeDcl, VisitScopedNames, effective_wire_name, is_skipped,
};
use serde_json::Value;
use std::collections::BTreeMap;

pub(crate) fn collect_types(spec: &Specification) -> BTreeMap<String, TypeSurface> {
    let mut types = BTreeMap::new();
    let mut modules = spec
        .0
        .iter()
        .filter_map(|definition| match definition {
            Definition::ModuleDcl(module) => Some(module),
            Definition::Pragma(_) => None,
            _ => None,
        })
        .collect::<Vec<_>>();
    let definitions = if modules.len() == 1
        && spec.0.iter().all(|definition| {
            matches!(definition, Definition::ModuleDcl(_) | Definition::Pragma(_))
        }) {
        modules
            .pop()
            .map(|module| {
                let mut definitions = module.definition.clone();
                definitions.remove_root_namespace(&module.ident);
                definitions
            })
            .unwrap_or_else(|| spec.0.clone())
    } else {
        spec.0.clone()
    };
    collect_definitions(&definitions, "", &mut types);
    types
}

fn collect_definitions(
    definitions: &[Definition],
    parent: &str,
    types: &mut BTreeMap<String, TypeSurface>,
) {
    for definition in definitions {
        match definition {
            Definition::ModuleDcl(module) => {
                collect_definitions(&module.definition, &join(parent, &module.ident), types)
            }
            Definition::ConstrTypeDcl(constr) => collect_constr(constr, parent, types),
            Definition::TypeDcl(TypeDcl::ConstrTypeDcl(constr)) => {
                collect_constr(constr, parent, types)
            }
            Definition::TypeDcl(TypeDcl::TypedefDcl(value)) => {
                for declarator in &value.decl {
                    types.insert(
                        join(parent, declarator_ident(declarator)),
                        TypeSurface {
                            kind: "typedef".to_string(),
                            details: typedef_details(value, declarator),
                            members: BTreeMap::new(),
                        },
                    );
                }
            }
            Definition::TypeDcl(TypeDcl::NativeDcl(value)) => {
                types.insert(
                    join(parent, &value.decl.0),
                    TypeSurface {
                        kind: "native".to_string(),
                        details: descriptor([("name", Value::String(value.decl.0.clone()))]),
                        members: BTreeMap::new(),
                    },
                );
            }
            Definition::ExceptDcl(value) => {
                types.insert(
                    join(parent, &value.ident),
                    TypeSurface {
                        kind: "exception".to_string(),
                        details: BTreeMap::new(),
                        members: collect_members(&value.member, &value.annotations),
                    },
                );
            }
            Definition::InterfaceDcl(interface) => {
                collect_interface_types(interface, parent, types)
            }
            Definition::Pragma(_) | Definition::ConstDcl(_) => {}
        }
    }
}

fn collect_interface_types(
    interface: &hir::InterfaceDcl,
    parent: &str,
    types: &mut BTreeMap<String, TypeSurface>,
) {
    let hir::InterfaceDclInner::InterfaceDef(definition) = &interface.decl else {
        return;
    };
    let interface_path = join(parent, &definition.header.ident);
    if let Some(body) = &definition.interface_body {
        for export in &body.0 {
            match export {
                hir::Export::TypeDcl(type_dcl) => match type_dcl {
                    TypeDcl::ConstrTypeDcl(constr) => {
                        collect_constr(constr, &interface_path, types)
                    }
                    TypeDcl::TypedefDcl(value) => {
                        for declarator in &value.decl {
                            types.insert(
                                join(&interface_path, declarator_ident(declarator)),
                                TypeSurface {
                                    kind: "typedef".to_string(),
                                    details: typedef_details(value, declarator),
                                    members: BTreeMap::new(),
                                },
                            );
                        }
                    }
                    TypeDcl::NativeDcl(value) => {
                        types.insert(
                            join(&interface_path, &value.decl.0),
                            TypeSurface {
                                kind: "native".to_string(),
                                details: descriptor([(
                                    "name",
                                    Value::String(value.decl.0.clone()),
                                )]),
                                members: BTreeMap::new(),
                            },
                        );
                    }
                },
                hir::Export::ExceptDcl(value) => {
                    types.insert(
                        join(&interface_path, &value.ident),
                        TypeSurface {
                            kind: "exception".to_string(),
                            details: BTreeMap::new(),
                            members: collect_members(&value.member, &value.annotations),
                        },
                    );
                }
                _ => {}
            }
        }
    }
}

fn collect_constr(
    constr: &hir::ConstrTypeDcl,
    parent: &str,
    types: &mut BTreeMap<String, TypeSurface>,
) {
    let (name, kind, details, members): TypeParts<'_> = match constr {
        hir::ConstrTypeDcl::StructDcl(value) => (
            value.ident.as_str(),
            "struct",
            BTreeMap::new(),
            collect_members(&value.member, &value.annotations),
        ),
        hir::ConstrTypeDcl::EnumDcl(value) => (
            value.ident.as_str(),
            "enum",
            BTreeMap::new(),
            value
                .member
                .iter()
                .enumerate()
                .filter_map(|(index, member)| {
                    if is_skipped(&member.annotations) {
                        return None;
                    }
                    Some((
                        effective_wire_name(&member.ident, &member.annotations, &value.annotations),
                        descriptor([
                            ("name", Value::String(member.ident.clone())),
                            ("index", Value::from(index)),
                        ]),
                    ))
                })
                .collect(),
        ),
        hir::ConstrTypeDcl::UnionDef(value) => (
            value.ident.as_str(),
            "union",
            descriptor([(
                "switch_type",
                Value::String(render_switch_type(&value.switch_type_spec)),
            )]),
            value
                .case
                .iter()
                .filter_map(|case| {
                    let ident = declarator_ident(&case.element.value).to_string();
                    if is_skipped(&case.element.annotations) {
                        return None;
                    }
                    let wire =
                        effective_wire_name(&ident, &case.element.annotations, &value.annotations);
                    let mut facts = declarator_facts(&case.element.value);
                    facts.insert(
                        "type".to_string(),
                        Value::String(render_element_type(&case.element.ty)),
                    );
                    facts.insert(
                        "labels".to_string(),
                        serde_json::to_value(&case.label).unwrap_or(Value::Null),
                    );
                    Some((wire, facts))
                })
                .collect(),
        ),
        hir::ConstrTypeDcl::BitsetDcl(value) => (
            value.ident.as_str(),
            "bitset",
            descriptor([(
                "parent",
                value
                    .parent
                    .as_ref()
                    .map(render_scoped_name)
                    .map(Value::String)
                    .unwrap_or(Value::Null),
            )]),
            value
                .field
                .iter()
                .map(|field| {
                    (
                        field.ident.clone(),
                        descriptor([
                            ("name", Value::String(field.ident.clone())),
                            (
                                "position",
                                serde_json::to_value(&field.pos.0).unwrap_or(Value::Null),
                            ),
                            (
                                "type",
                                field
                                    .ty
                                    .as_ref()
                                    .map(render_bitfield_type)
                                    .map(Value::String)
                                    .unwrap_or(Value::Null),
                            ),
                        ]),
                    )
                })
                .collect(),
        ),
        hir::ConstrTypeDcl::BitmaskDcl(value) => (
            value.ident.as_str(),
            "bitmask",
            BTreeMap::new(),
            value
                .value
                .iter()
                .filter(|field| !is_skipped(&field.annotations))
                .map(|field| {
                    (
                        effective_wire_name(&field.ident, &field.annotations, &value.annotations),
                        descriptor([
                            ("name", Value::String(field.ident.clone())),
                            ("position", Value::String(field.position.to_string())),
                        ]),
                    )
                })
                .collect(),
        ),
        hir::ConstrTypeDcl::StructForwardDcl(value) => (
            value.ident.as_str(),
            "struct",
            BTreeMap::new(),
            BTreeMap::new(),
        ),
        hir::ConstrTypeDcl::UnionForwardDcl(value) => (
            value.ident.as_str(),
            "union",
            BTreeMap::new(),
            BTreeMap::new(),
        ),
    };
    types.insert(
        join(parent, name),
        TypeSurface {
            kind: kind.to_string(),
            details,
            members,
        },
    );
}

fn typedef_details(
    value: &hir::TypedefDcl,
    declarator: &hir::Declarator,
) -> BTreeMap<String, Value> {
    let mut facts = declarator_facts(declarator);
    if matches!(declarator, hir::Declarator::SimpleDeclarator(_)) {
        facts.insert("array".to_string(), Value::Bool(false));
    }
    descriptor([
        (
            "type",
            Value::String(match &value.ty {
                hir::TypedefType::TypeSpec(value) => render_type(value),
                hir::TypedefType::ConstrTypeDcl(value) => value.ident().to_string(),
            }),
        ),
        (
            "declarators",
            Value::Array(vec![Value::Object(facts.into_iter().collect())]),
        ),
    ])
}

fn collect_members(
    members: &[hir::Member],
    container_annotations: &[hir::Annotation],
) -> BTreeMap<String, BTreeMap<String, Value>> {
    members
        .iter()
        .flat_map(|member| {
            member
                .ident
                .iter()
                .filter(|_| !is_skipped(&member.annotations))
                .map(move |declarator| (member, declarator))
        })
        .map(|(member, declarator)| {
            let ident = declarator_ident(declarator).to_string();
            let wire = effective_wire_name(&ident, &member.annotations, container_annotations);
            let mut facts = declarator_facts(declarator);
            facts.insert("type".to_string(), Value::String(render_type(&member.ty)));
            facts.insert("optional".to_string(), Value::Bool(member.is_optional()));
            if let Some(default) = member.default_value() {
                facts.insert("default".to_string(), Value::String(default));
            }
            (wire, facts)
        })
        .collect::<BTreeMap<_, _>>()
}
