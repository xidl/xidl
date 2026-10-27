//! Lower fully known integer constants in a selected API contract.

mod integer;
mod resolver;

use crate::error::ParserResult;
use crate::hir;

pub(super) use resolver::IntegerConstants;

/// Evaluates declarations after projection without changing source history.
pub(crate) struct IntegerDeclarations {
    constants: IntegerConstants,
}

impl IntegerDeclarations {
    /// Resolves local values while retaining symbols from independent imports.
    pub(crate) fn normalize(spec: &mut hir::Specification) -> ParserResult<()> {
        let mut declarations = Self {
            constants: IntegerConstants::collect(spec),
        };
        declarations.definitions(&mut spec.0, &[])
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
                hir::Definition::ConstDcl(constant) => {
                    self.constants.normalize_declaration(constant, scope)?;
                }
                hir::Definition::InterfaceDcl(interface) => {
                    if let hir::InterfaceDclInner::InterfaceDef(def) = &mut interface.decl
                        && let Some(body) = &mut def.interface_body
                    {
                        let mut nested = scope.to_vec();
                        nested.push(def.header.ident.clone());
                        for export in &mut body.0 {
                            if let hir::Export::ConstDcl(constant) = export {
                                self.constants.normalize_declaration(constant, &nested)?;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
