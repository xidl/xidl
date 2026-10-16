//! Collects declaration and member surfaces from projected HIR.

use super::super::helpers::descriptor;
use super::super::model::{TypeParts, TypeSurface};
use super::render::{
    join, render_bitfield_type, render_declarator, render_element_type, render_scoped_name,
    render_switch_type, render_type,
};
use crate::hir::{self, Definition, Specification, TypeDcl, effective_wire_name};
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
            .map(|module| module.definition.as_slice())
            .unwrap_or(spec.0.as_slice())
    } else {
        spec.0.as_slice()
    };
    collect_definitions(definitions, "", &mut types);
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
                let name = join(
                    parent,
                    &value
                        .decl
                        .first()
                        .map(render_declarator)
                        .unwrap_or_default(),
                );
                types.insert(
                    name,
                    TypeSurface {
                        kind: "typedef".to_string(),
                        details: typedef_details(value),
                        members: BTreeMap::new(),
                    },
                );
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
                hir::Export::TypeDcl(TypeDcl::ConstrTypeDcl(constr)) => {
                    collect_constr(constr, &interface_path, types)
                }
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
                .map(|(index, member)| {
                    (
                        effective_wire_name(&member.ident, &member.annotations, &value.annotations),
                        descriptor([
                            ("name", Value::String(member.ident.clone())),
                            ("index", Value::from(index)),
                        ]),
                    )
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
                .map(|case| {
                    let ident = render_declarator(&case.element.value);
                    (
                        ident.clone(),
                        descriptor([
                            ("name", Value::String(ident)),
                            ("type", Value::String(render_element_type(&case.element.ty))),
                            (
                                "labels",
                                serde_json::to_value(&case.label).unwrap_or(Value::Null),
                            ),
                        ]),
                    )
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
                .map(|field| {
                    (
                        field.ident.clone(),
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

fn typedef_details(value: &hir::TypedefDcl) -> BTreeMap<String, Value> {
    let declarators = value
        .decl
        .iter()
        .map(|declarator| match declarator {
            hir::Declarator::SimpleDeclarator(name) => descriptor([
                ("name", Value::String(name.0.clone())),
                ("array", Value::Bool(false)),
            ]),
            hir::Declarator::ArrayDeclarator(array) => descriptor([
                ("name", Value::String(array.ident.clone())),
                ("array", Value::Bool(true)),
                (
                    "lengths",
                    serde_json::to_value(&array.len).unwrap_or(Value::Null),
                ),
            ]),
        })
        .map(|value| Value::Object(value.into_iter().collect()))
        .collect::<Vec<_>>();
    descriptor([
        (
            "type",
            Value::String(match &value.ty {
                hir::TypedefType::TypeSpec(value) => render_type(value),
                hir::TypedefType::ConstrTypeDcl(value) => value.ident().to_string(),
            }),
        ),
        ("declarators", Value::Array(declarators)),
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
                .map(move |declarator| (member, declarator))
        })
        .map(|(member, declarator)| {
            let ident = render_declarator(declarator);
            let wire = effective_wire_name(&ident, &member.annotations, container_annotations);
            (
                wire,
                descriptor([
                    ("name", Value::String(ident)),
                    ("type", Value::String(render_type(&member.ty))),
                    ("optional", Value::Bool(member.is_optional())),
                ]),
            )
        })
        .collect::<BTreeMap<_, _>>()
}
