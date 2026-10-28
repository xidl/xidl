//! Resolve integer expression types and evaluate only required contract dimensions.

mod declarations;
mod expressions;
mod imports;
mod integer;
mod resolver;

use crate::error::ParserResult;
use crate::hir;

pub use imports::{INTEGER_IMPORTS_PROPERTY, IntegerExports, IntegerImports};
pub(super) use resolver::IntegerConstants;

/// Resolves declarations after projection without computing ordinary constants.
pub(crate) struct IntegerDeclarations {
    constants: IntegerConstants,
}

impl IntegerDeclarations {
    /// Resolves types using the selected contract and its imported declarations.
    pub(crate) fn normalize(
        spec: &mut hir::Specification,
        imports: &IntegerImports,
    ) -> ParserResult<()> {
        let mut declarations = Self {
            constants: IntegerConstants::with_imports(spec, imports),
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
