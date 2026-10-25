//! Collect complete typed shapes without inferring cross-version changes.

mod inheritance;
mod operations;
mod references;

use super::model::*;
use crate::error::{IdlcError, IdlcResult};
use std::collections::BTreeMap;
use xidl_parser::hir::{self, effective_wire_name, is_skipped};
use xidl_parser::rest_hir::{
    HttpOperation, HttpRequestBodyShape, HttpSignatureParamDirection, RestHirDocument,
};
use xidl_parser::versioning::map::VersionMap;

#[derive(Default)]
pub(super) struct SurfaceCollector {
    types: BTreeMap<String, TypeShape>,
    parents: BTreeMap<String, (Vec<String>, hir::ScopedName)>,
}

impl SurfaceCollector {
    pub(super) fn collect(
        http: &RestHirDocument,
        map: &VersionMap,
    ) -> IdlcResult<BTreeMap<String, VersionSurface>> {
        let mut result = BTreeMap::new();
        for version in &map.versions {
            let module = http
                .spec
                .0
                .iter()
                .find_map(|definition| match definition {
                    hir::Definition::ModuleDcl(module) if module.ident == version.id => {
                        Some(module)
                    }
                    _ => None,
                })
                .ok_or_else(|| {
                    IdlcError::fmt(format!(
                        "version adapters for {}: projected HIR is missing version {}",
                        map.contract, version.id
                    ))
                })?;
            let mut collector = Self::default();
            collector.definitions(&module.definition, &[]);
            collector.inherit(&version.id)?;
            let mut surface = VersionSurface {
                types: collector.types,
                ..VersionSurface::default()
            };
            for interface in &http.interfaces {
                if interface.module_path.first() != Some(&version.id) {
                    continue;
                }
                let scope = interface.module_path[1..].to_vec();
                let key = Self::join(&scope, &interface.name);
                for operation in &interface.operations {
                    surface.operations.insert(
                        format!("{key}.{}", operation.meta.name),
                        Self::operation(&key, &scope, operation)?,
                    );
                }
            }
            surface.localize_references(&version.id);
            result.insert(version.id.clone(), surface);
        }
        Ok(result)
    }

    fn definitions(&mut self, definitions: &[hir::Definition], scope: &[String]) {
        for definition in definitions {
            match definition {
                hir::Definition::ModuleDcl(module) => {
                    let mut nested = scope.to_vec();
                    nested.push(module.ident.clone());
                    self.definitions(&module.definition, &nested);
                }
                hir::Definition::ConstrTypeDcl(constr) => self.constr(constr, scope),
                hir::Definition::TypeDcl(decl) => self.type_decl(decl, scope),
                hir::Definition::ExceptDcl(decl) => {
                    self.types.insert(
                        Self::join(scope, &decl.ident),
                        TypeShape::Struct(Self::members(&decl.member, &decl.annotations, scope)),
                    );
                }
                hir::Definition::InterfaceDcl(interface) => {
                    let hir::InterfaceDclInner::InterfaceDef(def) = &interface.decl else {
                        continue;
                    };
                    let mut nested = scope.to_vec();
                    nested.push(def.header.ident.clone());
                    if let Some(body) = &def.interface_body {
                        for export in &body.0 {
                            match export {
                                hir::Export::TypeDcl(decl) => self.type_decl(decl, &nested),
                                hir::Export::ExceptDcl(decl) => {
                                    self.types.insert(
                                        Self::join(&nested, &decl.ident),
                                        TypeShape::Struct(Self::members(
                                            &decl.member,
                                            &decl.annotations,
                                            &nested,
                                        )),
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn type_decl(&mut self, decl: &hir::TypeDcl, scope: &[String]) {
        match decl {
            hir::TypeDcl::ConstrTypeDcl(constr) => self.constr(constr, scope),
            hir::TypeDcl::TypedefDcl(alias) => {
                for declarator in &alias.decl {
                    let name = Self::declarator_name(declarator);
                    let shape = match &alias.ty {
                        hir::TypedefType::TypeSpec(ty) => TypeShape::Alias(TypeUse {
                            ty: ty.clone(),
                            scope: scope.to_vec(),
                            declarator: declarator.clone(),
                            optional: false,
                            defaulted: false,
                            representation: ValueRepresentation::Model,
                        }),
                        hir::TypedefType::ConstrTypeDcl(_) => {
                            TypeShape::Unsupported("inline typedef".into())
                        }
                    };
                    self.types.insert(Self::join(scope, name), shape);
                }
            }
            hir::TypeDcl::NativeDcl(native) => {
                self.types.insert(
                    Self::join(scope, &native.decl.0),
                    TypeShape::Unsupported("native".into()),
                );
            }
        }
    }

    fn constr(&mut self, decl: &hir::ConstrTypeDcl, scope: &[String]) {
        let shape = match decl {
            hir::ConstrTypeDcl::StructDcl(value) => {
                if let Some(parent) = value.parent.first() {
                    self.parents.insert(
                        Self::join(scope, &value.ident),
                        (scope.to_vec(), parent.clone()),
                    );
                }
                TypeShape::Struct(Self::members(&value.member, &value.annotations, scope))
            }
            hir::ConstrTypeDcl::EnumDcl(value) => TypeShape::Enum(
                value
                    .member
                    .iter()
                    .filter(|member| !is_skipped(&member.annotations))
                    .map(|member| {
                        let wire = effective_wire_name(
                            &member.ident,
                            &member.annotations,
                            &value.annotations,
                        );
                        Member {
                            name: member.ident.clone(),
                            key: wire.clone(),
                            wire,
                            flatten: false,
                            path: Vec::new(),
                            value: None,
                        }
                    })
                    .collect(),
            ),
            hir::ConstrTypeDcl::UnionDef(value)
                if value.case.iter().any(|case| {
                    matches!(case.element.ty, hir::ElementSpecTy::ConstrTypeDcl(_))
                }) =>
            {
                TypeShape::Unsupported(
                    "inline union case: version_map does not expose its nested declaration facts"
                        .into(),
                )
            }
            hir::ConstrTypeDcl::UnionDef(value) => TypeShape::Union(
                value
                    .case
                    .iter()
                    .filter(|case| !is_skipped(&case.element.annotations))
                    .map(|case| {
                        let name = Self::declarator_name(&case.element.value).to_owned();
                        let wire = effective_wire_name(
                            &name,
                            &case.element.annotations,
                            &value.annotations,
                        );
                        let ty = match &case.element.ty {
                            hir::ElementSpecTy::TypeSpec(ty) => ty.clone(),
                            hir::ElementSpecTy::ConstrTypeDcl(ty) => {
                                hir::TypeSpec::ScopedName(hir::ScopedName {
                                    is_root: false,
                                    name: vec![match ty {
                                        hir::ConstrTypeDcl::StructDcl(value) => value.ident.clone(),
                                        hir::ConstrTypeDcl::StructForwardDcl(value) => {
                                            value.ident.clone()
                                        }
                                        hir::ConstrTypeDcl::EnumDcl(value) => value.ident.clone(),
                                        hir::ConstrTypeDcl::UnionDef(value) => value.ident.clone(),
                                        hir::ConstrTypeDcl::UnionForwardDcl(value) => {
                                            value.ident.clone()
                                        }
                                        hir::ConstrTypeDcl::BitsetDcl(value) => value.ident.clone(),
                                        hir::ConstrTypeDcl::BitmaskDcl(value) => {
                                            value.ident.clone()
                                        }
                                    }],
                                })
                            }
                        };
                        Member {
                            name,
                            key: wire.clone(),
                            wire,
                            flatten: false,
                            path: Vec::new(),
                            value: Some(TypeUse {
                                ty,
                                scope: scope.to_vec(),
                                declarator: case.element.value.clone(),
                                optional: false,
                                defaulted: false,
                                representation: ValueRepresentation::Model,
                            }),
                        }
                    })
                    .collect(),
            ),
            hir::ConstrTypeDcl::UnionForwardDcl(_) => {
                TypeShape::Unsupported("forward union declaration".into())
            }
            hir::ConstrTypeDcl::BitsetDcl(_) => TypeShape::Unsupported("bitset".into()),
            hir::ConstrTypeDcl::BitmaskDcl(_) => TypeShape::Unsupported("bitmask".into()),
            hir::ConstrTypeDcl::StructForwardDcl(_) => {
                TypeShape::Unsupported("forward declaration".into())
            }
        };
        let ident = match decl {
            hir::ConstrTypeDcl::StructDcl(value) => &value.ident,
            hir::ConstrTypeDcl::StructForwardDcl(value) => &value.ident,
            hir::ConstrTypeDcl::EnumDcl(value) => &value.ident,
            hir::ConstrTypeDcl::UnionDef(value) => &value.ident,
            hir::ConstrTypeDcl::UnionForwardDcl(value) => &value.ident,
            hir::ConstrTypeDcl::BitsetDcl(value) => &value.ident,
            hir::ConstrTypeDcl::BitmaskDcl(value) => &value.ident,
        };
        self.types.insert(Self::join(scope, ident), shape);
    }

    fn members(
        members: &[hir::Member],
        annotations: &[hir::Annotation],
        scope: &[String],
    ) -> Vec<Member> {
        members
            .iter()
            .filter(|member| !is_skipped(&member.annotations))
            .flat_map(|member| {
                member
                    .ident
                    .iter()
                    .map(move |declarator| (member, declarator))
            })
            .map(|(member, declarator)| {
                let name = Self::declarator_name(declarator).to_string();
                let wire = effective_wire_name(&name, &member.annotations, annotations);
                Member {
                    name,
                    key: wire.clone(),
                    wire,
                    path: Vec::new(),
                    flatten: member.annotations.iter().any(|annotation| {
                        hir::annotation_name(annotation)
                            .is_some_and(|name| name.eq_ignore_ascii_case("flatten"))
                    }),
                    value: Some(TypeUse {
                        ty: member.ty.clone(),
                        scope: scope.to_vec(),
                        declarator: declarator.clone(),
                        optional: member.is_optional(),
                        defaulted: member.default_value().is_some(),
                        representation: ValueRepresentation::Model,
                    }),
                }
            })
            .collect()
    }

    fn declarator_name(declarator: &hir::Declarator) -> &str {
        match declarator {
            hir::Declarator::SimpleDeclarator(name) => &name.0,
            hir::Declarator::ArrayDeclarator(array) => &array.ident,
        }
    }

    fn join(scope: &[String], name: &str) -> String {
        scope
            .iter()
            .map(String::as_str)
            .chain(std::iter::once(name))
            .collect::<Vec<_>>()
            .join("::")
    }
}
