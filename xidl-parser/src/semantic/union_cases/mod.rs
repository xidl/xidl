//! Validates symbolic union labels without changing their source spelling.

use crate::error::{ParseError, ParserResult};
use crate::hir::{
    CaseLabel, ConstExpr, ConstrTypeDcl, Definition, ElementSpecTy, Export, InterfaceDclInner,
    ScopedName, TypeDcl, TypedefType, UnionDef,
};
use std::collections::HashSet;

/// Checks union labels in source scopes before projection or interface expansion.
#[derive(Default)]
pub(crate) struct UnionCaseLabels<'a> {
    constants: HashSet<Vec<String>>,
    unions: Vec<(Vec<String>, &'a UnionDef)>,
}

#[cfg(test)]
mod tests;

impl<'a> UnionCaseLabels<'a> {
    /// Rejects bare enumerators while preserving references to declared constants.
    pub(crate) fn validate(definitions: &'a [Definition]) -> ParserResult<()> {
        let mut labels = Self::default();
        labels.collect(definitions, &[]);
        for (scope, union) in &labels.unions {
            labels.check_union(union, scope)?;
        }
        Ok(())
    }

    fn collect(&mut self, definitions: &'a [Definition], scope: &[String]) {
        for definition in definitions {
            match definition {
                Definition::ModuleDcl(module) => {
                    self.collect(&module.definition, &Self::path(scope, &module.ident));
                }
                Definition::TypeDcl(ty) => self.collect_type(ty, scope),
                Definition::ConstrTypeDcl(ty) => self.collect_constructed(ty, scope),
                Definition::ConstDcl(value) => {
                    self.constants.insert(Self::path(scope, &value.ident));
                }
                Definition::InterfaceDcl(interface) => {
                    if let InterfaceDclInner::InterfaceDef(definition) = &interface.decl
                        && let Some(body) = &definition.interface_body
                    {
                        let nested = Self::path(scope, &definition.header.ident);
                        for export in &body.0 {
                            self.collect_export(export, &nested);
                        }
                    }
                }
                Definition::ExceptDcl(_) | Definition::Pragma(_) => {}
            }
        }
    }

    fn collect_export(&mut self, export: &'a Export, scope: &[String]) {
        match export {
            Export::TypeDcl(ty) => self.collect_type(ty, scope),
            Export::ConstDcl(value) => {
                self.constants.insert(Self::path(scope, &value.ident));
            }
            Export::OpDcl(_) | Export::AttrDcl(_) | Export::ExceptDcl(_) => {}
        }
    }

    fn collect_type(&mut self, ty: &'a TypeDcl, scope: &[String]) {
        match ty {
            TypeDcl::ConstrTypeDcl(ty) => self.collect_constructed(ty, scope),
            TypeDcl::TypedefDcl(alias) => {
                if let TypedefType::ConstrTypeDcl(ty) = &alias.ty {
                    self.collect_constructed(ty, scope);
                }
            }
            TypeDcl::NativeDcl(_) => {}
        }
    }

    fn collect_constructed(&mut self, ty: &'a ConstrTypeDcl, scope: &[String]) {
        if let ConstrTypeDcl::UnionDef(union) = ty {
            self.unions.push((scope.to_vec(), union));
            let nested = Self::path(scope, &union.ident);
            for case in &union.case {
                if let ElementSpecTy::ConstrTypeDcl(ty) = &case.element.ty {
                    self.collect_constructed(ty, &nested);
                }
            }
        }
    }

    fn check_union(&self, union: &UnionDef, scope: &[String]) -> ParserResult<()> {
        for case in &union.case {
            for label in &case.label {
                let CaseLabel::Value(ConstExpr::ScopedName(name)) = label else {
                    continue;
                };
                if name.name.len() < 2 && !self.is_constant(name, scope) {
                    return Err(ParseError::Message(format!(
                        "union '{}': case label '{}' must be qualified as Enum::member",
                        Self::path(scope, &union.ident).join("::"),
                        name.name.join("::"),
                    )));
                }
            }
        }
        Ok(())
    }

    fn is_constant(&self, name: &ScopedName, scope: &[String]) -> bool {
        if name.is_root {
            return self.constants.contains(&name.name);
        }
        (0..=scope.len()).rev().any(|depth| {
            let mut path = scope[..depth].to_vec();
            path.extend_from_slice(&name.name);
            self.constants.contains(&path)
        })
    }

    fn path(scope: &[String], ident: &str) -> Vec<String> {
        let mut path = scope.to_vec();
        path.push(ident.to_string());
        path
    }
}
