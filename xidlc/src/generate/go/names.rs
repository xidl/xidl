//! IDL declarations and the names emitted by the Go model generator.

use std::collections::BTreeMap;

use xidl_parser::hir;

use super::definition::{constr_type_name, declarator_name, go_capitalize, go_export_name};
use crate::generate::utils::scope::resolve_canonical;

/// Go names indexed by their canonical IDL declaration paths.
#[derive(Default)]
pub(crate) struct GoNames(BTreeMap<String, String>);

impl GoNames {
    /// Collects named models and constants before resolving their uses.
    pub(crate) fn collect(spec: &hir::Specification) -> Self {
        let mut names = Self::default();
        names.definitions(&spec.0, &[]);
        names
    }

    /// Looks up the closest declaration using IDL lexical scope.
    pub(crate) fn resolve(&self, scope: &[String], name: &hir::ScopedName) -> Option<&str> {
        let key = resolve_canonical(scope, name, |key| self.0.contains_key(key))?;
        self.0.get(&key).map(String::as_str)
    }

    fn insert(&mut self, scope: &[String], ident: &str, output: String) {
        let key = scope
            .iter()
            .map(String::as_str)
            .chain(std::iter::once(ident))
            .collect::<Vec<_>>()
            .join("::");
        self.0.insert(key, output);
    }

    fn declaration(&mut self, scope: &[String], ident: &str) {
        self.insert(scope, ident, go_export_name(scope, ident));
    }

    fn definitions(&mut self, definitions: &[hir::Definition], scope: &[String]) {
        for definition in definitions {
            match definition {
                hir::Definition::ModuleDcl(module) => {
                    let mut nested = scope.to_vec();
                    nested.push(module.ident.clone());
                    self.definitions(&module.definition, &nested);
                }
                hir::Definition::TypeDcl(ty) => self.type_dcl(ty, scope),
                hir::Definition::ConstrTypeDcl(ty) => self.constructed(ty, scope),
                hir::Definition::ConstDcl(value) => self.declaration(scope, &value.ident),
                hir::Definition::ExceptDcl(value) => self.declaration(scope, &value.ident),
                hir::Definition::InterfaceDcl(interface) => {
                    let hir::InterfaceDclInner::InterfaceDef(def) = &interface.decl else {
                        continue;
                    };
                    self.declaration(scope, &def.header.ident);
                    let mut nested = scope.to_vec();
                    nested.push(def.header.ident.clone());
                    if let Some(body) = &def.interface_body {
                        for export in &body.0 {
                            match export {
                                hir::Export::TypeDcl(ty) => self.type_dcl(ty, &nested),
                                hir::Export::ConstDcl(value) => {
                                    self.declaration(&nested, &value.ident)
                                }
                                hir::Export::ExceptDcl(value) => {
                                    self.declaration(&nested, &value.ident)
                                }
                                hir::Export::OpDcl(_) | hir::Export::AttrDcl(_) => {}
                            }
                        }
                    }
                }
                hir::Definition::Pragma(_) => {}
            }
        }
    }

    fn type_dcl(&mut self, ty: &hir::TypeDcl, scope: &[String]) {
        match ty {
            hir::TypeDcl::ConstrTypeDcl(ty) => self.constructed(ty, scope),
            hir::TypeDcl::TypedefDcl(alias) => {
                if let hir::TypedefType::ConstrTypeDcl(ty) = &alias.ty {
                    self.constructed(ty, scope);
                }
                for decl in &alias.decl {
                    self.declaration(scope, declarator_name(decl));
                }
            }
            hir::TypeDcl::NativeDcl(value) => self.declaration(scope, &value.decl.0),
        }
    }

    fn constructed(&mut self, ty: &hir::ConstrTypeDcl, scope: &[String]) {
        let name = constr_type_name(ty, scope);
        let ident = match ty {
            hir::ConstrTypeDcl::StructDcl(value) => &value.ident,
            hir::ConstrTypeDcl::StructForwardDcl(value) => &value.ident,
            hir::ConstrTypeDcl::EnumDcl(value) => &value.ident,
            hir::ConstrTypeDcl::UnionDef(value) => &value.ident,
            hir::ConstrTypeDcl::UnionForwardDcl(value) => &value.ident,
            hir::ConstrTypeDcl::BitsetDcl(value) => &value.ident,
            hir::ConstrTypeDcl::BitmaskDcl(value) => &value.ident,
        };
        self.insert(scope, ident, name.clone());
        let mut nested = scope.to_vec();
        nested.push(ident.clone());
        match ty {
            hir::ConstrTypeDcl::EnumDcl(value) => {
                for member in &value.member {
                    self.insert(
                        &nested,
                        &member.ident,
                        format!("{name}{}", go_capitalize(&member.ident)),
                    );
                }
            }
            hir::ConstrTypeDcl::UnionDef(value) => {
                for case in &value.case {
                    if let hir::ElementSpecTy::ConstrTypeDcl(ty) = &case.element.ty {
                        self.constructed(ty, &nested);
                    }
                }
            }
            _ => {}
        }
    }
}
