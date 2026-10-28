//! Immutable source declarations indexed in their original lexical scope.

use crate::hir::{self, DeclarationNames};
use std::collections::BTreeMap;

/// Integer declarations and the names that can shadow their references.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct IntegerIndex {
    pub(super) names: DeclarationNames,
    pub(super) declarations: BTreeMap<String, hir::ConstDcl>,
    pub(super) aliases: BTreeMap<String, hir::TypeSpec>,
}

impl IntegerIndex {
    /// Collects source declarations before any expression is evaluated.
    pub(super) fn collect(spec: &hir::Specification) -> Self {
        let mut index = Self {
            names: DeclarationNames::collect(&spec.0),
            declarations: BTreeMap::new(),
            aliases: BTreeMap::new(),
        };
        index.definitions(&spec.0, &[]);
        index
    }

    fn definitions(&mut self, definitions: &[hir::Definition], scope: &[String]) {
        for definition in definitions {
            match definition {
                hir::Definition::ModuleDcl(module) => {
                    let mut nested = scope.to_vec();
                    nested.push(module.ident.clone());
                    self.definitions(&module.definition, &nested);
                }
                hir::Definition::ConstDcl(constant) => self.insert(constant, scope),
                hir::Definition::TypeDcl(decl) => self.alias(decl, scope),
                hir::Definition::InterfaceDcl(interface) => {
                    if let hir::InterfaceDclInner::InterfaceDef(def) = &interface.decl
                        && let Some(body) = &def.interface_body
                    {
                        let mut nested = scope.to_vec();
                        nested.push(def.header.ident.clone());
                        for export in &body.0 {
                            match export {
                                hir::Export::ConstDcl(constant) => self.insert(constant, &nested),
                                hir::Export::TypeDcl(decl) => self.alias(decl, &nested),
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn key(scope: &[String], name: &str) -> String {
        scope
            .iter()
            .map(String::as_str)
            .chain([name])
            .collect::<Vec<_>>()
            .join("::")
    }

    fn insert(&mut self, constant: &hir::ConstDcl, scope: &[String]) {
        self.declarations
            .insert(Self::key(scope, &constant.ident), constant.clone());
    }

    fn alias(&mut self, decl: &hir::TypeDcl, scope: &[String]) {
        if let hir::TypeDcl::TypedefDcl(alias) = decl
            && let hir::TypedefType::TypeSpec(ty) = &alias.ty
        {
            for declarator in &alias.decl {
                if let hir::Declarator::SimpleDeclarator(name) = declarator {
                    self.aliases.insert(Self::key(scope, &name.0), ty.clone());
                }
            }
        }
    }
}
