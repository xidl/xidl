//! Preserve the ordinary Rust model's layout facts from the public HIR.

use crate::error::{IdlcError, IdlcResult};
use crate::generate::rust::util::declarator_name;
use std::collections::{BTreeMap, BTreeSet};
use xidl_parser::hir;

pub(super) enum Declaration<'a> {
    Struct(&'a hir::StructDcl),
    Exception(&'a hir::ExceptDcl),
    Union(&'a hir::UnionDef),
    Enum(&'a hir::EnumDcl),
    Alias,
}

#[derive(Default)]
pub(super) struct Layout<'a> {
    pub declarations: BTreeMap<String, Declaration<'a>>,
    pub constants: BTreeSet<String>,
}

impl<'a> Layout<'a> {
    pub(super) fn new(spec: &'a hir::Specification) -> Self {
        let mut layout = Self::default();
        layout.definitions(&spec.0, &[]);
        layout
    }

    fn definitions(&mut self, definitions: &'a [hir::Definition], scope: &[String]) {
        for definition in definitions {
            match definition {
                hir::Definition::ModuleDcl(module) => {
                    let mut scope = scope.to_vec();
                    scope.push(module.ident.clone());
                    self.definitions(&module.definition, &scope);
                }
                hir::Definition::ConstrTypeDcl(declaration) => self.constr(declaration, scope),
                hir::Definition::TypeDcl(declaration) => self.ty(declaration, scope),
                hir::Definition::ExceptDcl(declaration) => {
                    self.declarations.insert(
                        Self::key(scope, &declaration.ident),
                        Declaration::Exception(declaration),
                    );
                }
                hir::Definition::ConstDcl(declaration) => {
                    self.constants.insert(Self::key(scope, &declaration.ident));
                }
                hir::Definition::InterfaceDcl(declaration) => {
                    let hir::InterfaceDclInner::InterfaceDef(interface) = &declaration.decl else {
                        continue;
                    };
                    let mut scope = scope.to_vec();
                    scope.push(interface.header.ident.clone());
                    if let Some(body) = &interface.interface_body {
                        for export in &body.0 {
                            match export {
                                hir::Export::TypeDcl(declaration) => self.ty(declaration, &scope),
                                hir::Export::ExceptDcl(declaration) => {
                                    self.declarations.insert(
                                        Self::key(&scope, &declaration.ident),
                                        Declaration::Exception(declaration),
                                    );
                                }
                                hir::Export::ConstDcl(declaration) => {
                                    self.constants.insert(Self::key(&scope, &declaration.ident));
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

    fn constr(&mut self, declaration: &'a hir::ConstrTypeDcl, scope: &[String]) {
        let (name, declaration) = match declaration {
            hir::ConstrTypeDcl::StructDcl(value) => (&value.ident, Declaration::Struct(value)),
            hir::ConstrTypeDcl::EnumDcl(value) => (&value.ident, Declaration::Enum(value)),
            hir::ConstrTypeDcl::UnionDef(value) => (&value.ident, Declaration::Union(value)),
            _ => return,
        };
        self.declarations
            .insert(Self::key(scope, name), declaration);
    }

    fn ty(&mut self, declaration: &'a hir::TypeDcl, scope: &[String]) {
        match declaration {
            hir::TypeDcl::ConstrTypeDcl(value) => self.constr(value, scope),
            hir::TypeDcl::TypedefDcl(value) => {
                for declarator in &value.decl {
                    self.declarations.insert(
                        Self::key(scope, &declarator_name(declarator)),
                        Declaration::Alias,
                    );
                }
            }
            _ => {}
        }
    }

    pub(super) fn members(&self, key: &str) -> IdlcResult<&[hir::Member]> {
        let members = match self.declarations.get(key) {
            Some(Declaration::Struct(value)) => value.member.as_slice(),
            Some(Declaration::Exception(value)) => value.member.as_slice(),
            _ => {
                return Err(Self::error(
                    key,
                    "expected a Rust struct or exception layout",
                ));
            }
        };
        if members
            .iter()
            .any(|member| hir::is_skipped(&member.annotations))
        {
            return Err(Self::error(
                key,
                "skipped model fields require an explicit whole-value conversion",
            ));
        }
        Ok(members)
    }

    pub(super) fn validate(&self, key: &str) -> IdlcResult<()> {
        if let Some(parent) = self.parent(key)? {
            self.validate(&parent)?;
        }
        let skipped = match self.declarations.get(key) {
            Some(Declaration::Struct(_) | Declaration::Exception(_)) => {
                self.members(key)?;
                false
            }
            Some(Declaration::Enum(value)) => value
                .member
                .iter()
                .any(|member| hir::is_skipped(&member.annotations)),
            Some(Declaration::Union(value)) => value
                .case
                .iter()
                .any(|case| hir::is_skipped(&case.element.annotations)),
            Some(Declaration::Alias) => false,
            None => return Err(Self::error(key, "missing public model layout")),
        };
        if skipped {
            return Err(Self::error(
                key,
                "skipped model cases have no explicit adapter representation",
            ));
        }
        Ok(())
    }

    pub(super) fn recursive(&self, key: &str, member: &str) -> IdlcResult<bool> {
        Ok(self.members(key)?.iter().any(|value| {
            value.recursive
                && value
                    .ident
                    .iter()
                    .any(|decl| declarator_name(decl) == member)
        }))
    }

    pub(super) fn key(scope: &[String], name: &str) -> String {
        scope
            .iter()
            .map(String::as_str)
            .chain(std::iter::once(name))
            .collect::<Vec<_>>()
            .join("::")
    }

    pub(super) fn error(key: &str, reason: &str) -> IdlcError {
        IdlcError::rpc(format!("rust-axum adapters for {key}: {reason}"))
    }
}
