//! Scoped references carried by HTTP operation types.

use super::*;
use hir::{ScopedName, VisitScopedNames};

impl VisitScopedNames for HttpOperation {
    fn visit_scoped_names(&mut self, visit: &mut impl FnMut(&mut ScopedName)) {
        for parameter in &mut self.signature.params {
            parameter.ty.visit_scoped_names(visit);
        }
        if let Some(ty) = &mut self.signature.return_type {
            ty.visit_scoped_names(visit);
        }
        for bindings in [
            &mut self.http.request.path,
            &mut self.http.request.query,
            &mut self.http.request.header,
            &mut self.http.request.cookie,
        ] {
            for binding in bindings {
                binding.ty.visit_scoped_names(visit);
            }
        }
        match &mut self.http.request.body.shape {
            HttpRequestBodyShape::Empty => {}
            HttpRequestBodyShape::SingleValue { ty, .. } => ty.visit_scoped_names(visit),
            HttpRequestBodyShape::Object { fields } => {
                for field in fields {
                    field.ty.visit_scoped_names(visit);
                }
            }
            HttpRequestBodyShape::Stream { item_ty, .. } => item_ty.visit_scoped_names(visit),
        }
        for bindings in [
            &mut self.http.response.header,
            &mut self.http.response.cookie,
        ] {
            for binding in bindings {
                binding.ty.visit_scoped_names(visit);
            }
        }
        match &mut self.http.response.body.shape {
            HttpResponseBodyShape::Empty => {}
            HttpResponseBodyShape::ReturnOnly { ty }
            | HttpResponseBodyShape::SingleValue { ty, .. } => ty.visit_scoped_names(visit),
            HttpResponseBodyShape::Object { fields } => {
                for field in fields {
                    field.ty.visit_scoped_names(visit);
                }
            }
            HttpResponseBodyShape::Stream { item_ty, .. } => item_ty.visit_scoped_names(visit),
        }
        for representation in &mut self.http.response.representations {
            representation.ty.visit_scoped_names(visit);
        }
    }
}
