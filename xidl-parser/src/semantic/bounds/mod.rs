//! Resolve collection bounds before targets consume the shared HIR.

use super::constants::{IntegerConstants, IntegerImports};
use crate::error::{ParseError, ParserResult};
use crate::hir;

/// Normalizes exact array sizes and maximum collection lengths in shared HIR.
pub(crate) struct CollectionBounds {
    constants: IntegerConstants,
}

impl CollectionBounds {
    /// Computes dimensions in the selected projection before inheritance copies them.
    pub(crate) fn normalize(
        spec: &mut hir::Specification,
        imports: &IntegerImports,
    ) -> ParserResult<()> {
        let mut dimensions = Self {
            constants: IntegerConstants::with_imports(spec, imports),
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
                hir::Export::OpDcl(op) => {
                    if let hir::OpTypeSpec::TypeSpec(ty) = &mut op.ty {
                        self.type_spec(ty, scope)?;
                    }
                    if let Some(params) = &mut op.parameter {
                        for param in &mut params.0 {
                            self.type_spec(&mut param.ty, scope)?;
                        }
                    }
                }
                hir::Export::AttrDcl(attr) => match &mut attr.decl {
                    hir::AttrDclInner::ReadonlyAttrSpec(value) => {
                        self.type_spec(&mut value.ty, scope)?
                    }
                    hir::AttrDclInner::AttrSpec(value) => self.type_spec(&mut value.ty, scope)?,
                },
                hir::Export::ConstDcl(_) => {}
            }
        }
        Ok(())
    }

    fn type_decl(&mut self, decl: &mut hir::TypeDcl, scope: &[String]) -> ParserResult<()> {
        match decl {
            hir::TypeDcl::ConstrTypeDcl(constr) => self.constructed(constr, scope)?,
            hir::TypeDcl::TypedefDcl(alias) => {
                match &mut alias.ty {
                    hir::TypedefType::ConstrTypeDcl(constr) => self.constructed(constr, scope)?,
                    hir::TypedefType::TypeSpec(ty) => self.type_spec(ty, scope)?,
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
                    match &mut case.element.ty {
                        hir::ElementSpecTy::ConstrTypeDcl(constr) => {
                            self.constructed(constr, scope)?
                        }
                        hir::ElementSpecTy::TypeSpec(ty) => self.type_spec(ty, scope)?,
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
            self.type_spec(&mut member.ty, scope)?;
            for decl in &mut member.ident {
                self.declarator(decl, scope, owner)?;
            }
        }
        Ok(())
    }

    fn type_spec(&mut self, ty: &mut hir::TypeSpec, scope: &[String]) -> ParserResult<()> {
        match ty {
            hir::TypeSpec::SequenceType(sequence) => {
                if let Some(bound) = &mut sequence.len {
                    self.bound(bound, scope, "sequence bound")?;
                }
                self.type_spec(&mut sequence.ty, scope)?;
            }
            hir::TypeSpec::MapType(map) => {
                if let Some(bound) = &mut map.len {
                    self.bound(bound, scope, "map bound")?;
                }
                self.type_spec(&mut map.key, scope)?;
                self.type_spec(&mut map.value, scope)?;
            }
            hir::TypeSpec::TemplateType(template) => {
                for arg in &mut template.args {
                    self.type_spec(arg, scope)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn bound(
        &mut self,
        bound: &mut hir::PositiveIntConst,
        scope: &[String],
        contract: &str,
    ) -> ParserResult<()> {
        let size = self
            .constants
            .dimension(&bound.0, scope)
            .map_err(|error| ParseError::Message(format!("{contract}: {error}")))?;
        if size <= 0 {
            return Err(ParseError::Message(format!(
                "{contract} must be positive, got {size}"
            )));
        }
        bound.0 = hir::ConstExpr::Literal(hir::Literal::IntegerLiteral(hir::IntegerLiteral(
            size.to_string(),
        )));
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
            self.bound(length, scope, &format!("array dimension for '{location}'"))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
