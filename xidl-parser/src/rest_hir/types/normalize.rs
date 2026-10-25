//! Keep declaration identity in logical signatures and resolve text wire shapes.

use super::{DeclarationKind, TypeDeclarations};
use crate::hir::{self, VisitScopedNames};
use crate::rest_hir::{HttpInterface, HttpOperation, RestHirResult};

impl TypeDeclarations<'_> {
    pub(in crate::rest_hir) fn normalize(
        &self,
        interfaces: &mut [HttpInterface],
    ) -> RestHirResult<()> {
        for interface in interfaces {
            let mut scope = interface.module_path.clone();
            scope.push(interface.name.clone());
            for operation in &mut interface.operations {
                self.operation(operation, &scope)?;
            }
        }
        Ok(())
    }

    fn operation(&self, operation: &mut HttpOperation, scope: &[String]) -> RestHirResult<()> {
        operation.visit_types(&mut |ty| *ty = self.qualified(ty, scope));
        let request = &mut operation.http.request;
        for binding in request
            .query
            .iter_mut()
            .chain(&mut request.header)
            .chain(&mut request.cookie)
        {
            binding.ty = self
                .input_type(&binding.ty, scope, &mut Vec::new())
                .map_err(|error| {
                    format!(
                        "HTTP input '{}.{}::{}': {error}",
                        scope.join("::"),
                        operation.meta.name,
                        binding.source_param
                    )
                })?;
        }
        Ok(())
    }

    pub(super) fn qualified(&self, ty: &hir::TypeSpec, scope: &[String]) -> hir::TypeSpec {
        let mut qualified = ty.clone();
        qualified.visit_scoped_names(&mut |name| {
            if let Some(declaration) = self.resolve(name, scope)
                && !matches!(declaration.kind, DeclarationKind::Other)
            {
                name.name = declaration
                    .module_path
                    .iter()
                    .cloned()
                    .chain(std::iter::once(declaration.ident.to_string()))
                    .collect();
                name.is_root = true;
            }
        });
        qualified
    }
}
