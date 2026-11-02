use crate::error::{DocumentationError, Result};
use serde_json::Value;
use xidl_parser::{hir, rest_hir::semantics::deprecated_info};
use xidlc::generate::openapi::OpenApiProjection;

#[derive(Default)]
pub(super) struct Metadata {
    pub annotations: Vec<hir::Annotation>,
    pub ty: Option<hir::TypeSpec>,
    pub declared_default: Option<String>,
    pub array_depth: usize,
}

impl Metadata {
    pub fn apply(&self, schema: &mut Value) -> Result<()> {
        let text = OpenApiProjection::documentation(&self.annotations);
        if !text.is_empty() {
            schema["description"] = text.into();
        }
        if let Some(info) =
            deprecated_info(&self.annotations).map_err(DocumentationError::Contract)?
        {
            schema["deprecated"] = true.into();
            schema["x-xidl-deprecation"] = serde_json::to_value(info)?;
        }
        let declared_default = self.declared_default.as_deref().or_else(|| {
            self.annotations
                .iter()
                .find_map(|annotation| match annotation {
                    hir::Annotation::Default { value } => Some(value.as_str()),
                    _ => None,
                })
        });
        if let Some(value) = declared_default {
            schema["x-xidl-declared-default"] = value.into();
        }

        let constraints = self
            .annotations
            .iter()
            .filter(|annotation| {
                matches!(
                    annotation,
                    hir::Annotation::Range { .. }
                        | hir::Annotation::Min { .. }
                        | hir::Annotation::Max { .. }
                )
            })
            .collect::<Vec<_>>();
        if !constraints.is_empty() {
            schema["x-xidl-declared-constraints"] = serde_json::to_value(constraints)?;
        }
        if let Some(ty) = &self.ty {
            let mut inner = schema;
            for _ in 0..self.array_depth {
                inner = &mut inner["items"];
            }
            Self::bounds(ty, inner);
        }
        Ok(())
    }

    pub fn bounds(ty: &hir::TypeSpec, schema: &mut Value) {
        let bound = match ty {
            hir::TypeSpec::StringType(ty) => ty.bound.as_ref(),
            hir::TypeSpec::WideStringType(ty) => ty.bound.as_ref(),
            hir::TypeSpec::SequenceType(ty) => {
                Self::bounds(&ty.ty, &mut schema["items"]);
                return;
            }
            hir::TypeSpec::MapType(ty) => {
                Self::bounds(&ty.value, &mut schema["additionalProperties"]);
                return;
            }
            _ => return,
        };
        if let Some(bound) = bound.and_then(|bound| hir::const_expr_to_i64(&bound.0)) {
            schema["maxLength"] = bound.into();
        }
    }

    pub fn declarator(declarator: &hir::Declarator) -> (&str, usize) {
        match declarator {
            hir::Declarator::SimpleDeclarator(name) => (&name.0, 0),
            hir::Declarator::ArrayDeclarator(array) => (&array.ident, array.len.len()),
        }
    }
}
