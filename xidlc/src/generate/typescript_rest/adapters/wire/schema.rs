//! Reuse normal TypeScript scalar and container rules with wire references.

use super::*;
use crate::generate::adapter::{TypeUse, ValueRepresentation};
use crate::generate::typescript::definition::type_expr::zod_schema_for_decl;
use xidl_parser::hir;

impl WireDecoderBuilder<'_> {
    pub(super) fn schema(&mut self, version: &str, value: &TypeUse) -> IdlcResult<ZodSchema> {
        if value.representation == ValueRepresentation::ByteBuffer {
            return Ok(ZodSchema::Primitive("instanceof(Uint8Array)".into()));
        }
        let mut ty = value.ty.clone();
        self.normalize(&mut ty, version, &value.scope)?;
        Ok(zod_schema_for_decl(&ty, &value.declarator, &[]))
    }

    fn normalize(
        &mut self,
        ty: &mut hir::TypeSpec,
        version: &str,
        scope: &[String],
    ) -> IdlcResult<()> {
        match ty {
            hir::TypeSpec::ScopedName(name) => {
                let key = self.layouts(version)?.resolve(scope, name)?;
                name.name = vec![self.ensure(version, &key)?];
                // Wire schemas are local declarations in this adapter file.
                name.is_root = false;
            }
            hir::TypeSpec::SequenceType(sequence) => {
                self.normalize(&mut sequence.ty, version, scope)?
            }
            hir::TypeSpec::MapType(map) => {
                self.normalize(&mut map.key, version, scope)?;
                self.normalize(&mut map.value, version, scope)?;
            }
            hir::TypeSpec::TemplateType(_) => {
                return Err(Self::error(
                    version,
                    "custom templates have no declared wire validation",
                ));
            }
            _ => {}
        }
        Ok(())
    }
}
