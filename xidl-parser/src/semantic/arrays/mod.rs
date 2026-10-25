//! Resolve fixed-array dimensions before targets consume the shared HIR.

mod constants;
mod integer;

use crate::error::{ParseError, ParserResult};
use crate::hir;
use constants::IntegerConstants;

/// Normalizes array sizes without changing public constant declarations.
pub(super) struct ArrayDimensions {
    constants: IntegerConstants,
}

impl ArrayDimensions {
    /// Computes dimensions in the selected projection before inheritance copies them.
    pub(super) fn normalize(spec: &mut hir::Specification) -> ParserResult<()> {
        let mut dimensions = Self {
            constants: IntegerConstants::collect(spec),
        };
        dimensions.definitions(&mut spec.0, &[])
    }

    fn definitions(
        &mut self,
        definitions: &mut [hir::Definition],
        scope: &[String],
    ) -> ParserResult<()> {
        for definition in definitions {
            match definition {
                hir::Definition::ModuleDcl(module) => {
                    let mut nested = scope.to_vec();
                    nested.push(module.ident.clone());
                    self.definitions(&mut module.definition, &nested)?;
                }
                hir::Definition::TypeDcl(decl) => self.type_decl(decl, scope)?,
                hir::Definition::ConstrTypeDcl(constr) => self.constructed(constr, scope)?,
                hir::Definition::ExceptDcl(except) => {
                    self.members(&mut except.member, scope, &except.ident)?;
                }
                hir::Definition::InterfaceDcl(interface) => {
                    if let hir::InterfaceDclInner::InterfaceDef(def) = &mut interface.decl
                        && let Some(body) = &mut def.interface_body
                    {
                        let mut nested = scope.to_vec();
                        nested.push(def.header.ident.clone());
                        self.exports(&mut body.0, &nested)?;
                    }
                }
                hir::Definition::ConstDcl(_) | hir::Definition::Pragma(_) => {}
            }
        }
        Ok(())
    }

    fn exports(&mut self, exports: &mut [hir::Export], scope: &[String]) -> ParserResult<()> {
        for export in exports {
            match export {
                hir::Export::TypeDcl(decl) => self.type_decl(decl, scope)?,
                hir::Export::ExceptDcl(except) => {
                    self.members(&mut except.member, scope, &except.ident)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn type_decl(&mut self, decl: &mut hir::TypeDcl, scope: &[String]) -> ParserResult<()> {
        match decl {
            hir::TypeDcl::ConstrTypeDcl(constr) => self.constructed(constr, scope)?,
            hir::TypeDcl::TypedefDcl(alias) => {
                if let hir::TypedefType::ConstrTypeDcl(constr) = &mut alias.ty {
                    self.constructed(constr, scope)?;
                }
                for decl in &mut alias.decl {
                    self.declarator(decl, scope, "")?;
                }
            }
            hir::TypeDcl::NativeDcl(_) => {}
        }
        Ok(())
    }

    fn constructed(
        &mut self,
        constr: &mut hir::ConstrTypeDcl,
        scope: &[String],
    ) -> ParserResult<()> {
        match constr {
            hir::ConstrTypeDcl::StructDcl(def) => {
                self.members(&mut def.member, scope, &def.ident)?;
                self.members(&mut def.inherited, scope, &def.ident)?;
            }
            hir::ConstrTypeDcl::UnionDef(union) => {
                for case in &mut union.case {
                    if let hir::ElementSpecTy::ConstrTypeDcl(constr) = &mut case.element.ty {
                        self.constructed(constr, scope)?;
                    }
                    self.declarator(&mut case.element.value, scope, &union.ident)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn members(
        &mut self,
        members: &mut [hir::Member],
        scope: &[String],
        owner: &str,
    ) -> ParserResult<()> {
        for member in members {
            for decl in &mut member.ident {
                self.declarator(decl, scope, owner)?;
            }
        }
        Ok(())
    }

    fn declarator(
        &mut self,
        decl: &mut hir::Declarator,
        scope: &[String],
        owner: &str,
    ) -> ParserResult<()> {
        let hir::Declarator::ArrayDeclarator(array) = decl else {
            return Ok(());
        };
        let location = scope
            .iter()
            .map(String::as_str)
            .chain([owner, array.ident.as_str()])
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("::");
        for length in &mut array.len {
            let size = self
                .constants
                .dimension(&length.0, scope)
                .map_err(|error| {
                    ParseError::Message(format!("array dimension for '{location}': {error}"))
                })?;
            if size <= 0 {
                return Err(ParseError::Message(format!(
                    "array dimension for '{location}' must be positive, got {size}"
                )));
            }
            length.0 = hir::ConstExpr::Literal(hir::Literal::IntegerLiteral(hir::IntegerLiteral(
                size.to_string(),
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
