//! Expand transparent input aliases without treating fixed arrays as sequences.

use super::{DeclarationKind, TypeDeclarations};
use crate::hir;
use crate::rest_hir::RestHirResult;

impl TypeDeclarations<'_> {
    pub(super) fn input_type(
        &self,
        ty: &hir::TypeSpec,
        scope: &[String],
        visiting: &mut Vec<Vec<String>>,
    ) -> RestHirResult<hir::TypeSpec> {
        match ty {
            hir::TypeSpec::ScopedName(name) => {
                let Some(declaration) = self.resolve(name, scope) else {
                    return Ok(ty.clone());
                };
                let DeclarationKind::Alias(target) = declaration.kind else {
                    return Ok(self.qualified(ty, scope));
                };
                let path = declaration
                    .module_path
                    .iter()
                    .cloned()
                    .chain(std::iter::once(declaration.ident.to_string()))
                    .collect::<Vec<_>>();
                if visiting.contains(&path) {
                    return Err(format!("typedef cycle includes '{}'", path.join("::")));
                }
                visiting.push(path);
                let resolved = self.input_type(target, &declaration.module_path, visiting);
                visiting.pop();
                resolved
            }
            hir::TypeSpec::SequenceType(sequence) => {
                Ok(hir::TypeSpec::SequenceType(hir::SequenceType {
                    ty: Box::new(self.input_type(&sequence.ty, scope, visiting)?),
                    len: sequence.len.clone(),
                }))
            }
            _ => Ok(ty.clone()),
        }
    }
}
