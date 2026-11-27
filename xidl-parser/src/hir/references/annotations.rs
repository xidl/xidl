use super::{ReferenceVisitor, WalkScopedNames};
use crate::hir::{Annotation, AnnotationParams, ScopedName};

impl<F: FnMut(&mut ScopedName)> ReferenceVisitor<F> {
    pub(super) fn annotations(&mut self, annotations: &mut [Annotation]) {
        if self.include_annotations {
            annotations.walk_scoped_names(self);
        }
    }
}

impl WalkScopedNames for Annotation {
    fn walk_scoped_names(&mut self, visitor: &mut ReferenceVisitor<impl FnMut(&mut ScopedName)>) {
        if !visitor.include_annotations {
            return;
        }
        let (Self::Builtin { params, .. } | Self::ScopedName { params, .. }) = self else {
            return;
        };
        match params {
            Some(AnnotationParams::ConstExpr(expr)) => expr.walk_scoped_names(visitor),
            Some(AnnotationParams::Positional(values)) => values.walk_scoped_names(visitor),
            Some(AnnotationParams::Params(params)) => {
                for param in params {
                    if let Some(value) = &mut param.value {
                        value.walk_scoped_names(visitor);
                    }
                }
            }
            Some(AnnotationParams::Raw(_)) | None => {}
        }
    }
}
