//! Traverse logical and transport type references together.

use super::{HttpOperation, HttpRequestBodyShape, HttpResponseBodyShape};
use crate::hir;

impl HttpOperation {
    /// Visits every signature, binding, body and representation type in an operation.
    pub fn visit_types(&mut self, visit: &mut impl FnMut(&mut hir::TypeSpec)) {
        for parameter in &mut self.signature.params {
            visit(&mut parameter.ty);
        }
        if let Some(ty) = &mut self.signature.return_type {
            visit(ty);
        }
        let request = &mut self.http.request;
        for binding in request
            .path
            .iter_mut()
            .chain(&mut request.query)
            .chain(&mut request.header)
            .chain(&mut request.cookie)
        {
            visit(&mut binding.ty);
        }
        match &mut request.body.shape {
            HttpRequestBodyShape::SingleValue { ty, .. }
            | HttpRequestBodyShape::Stream { item_ty: ty, .. } => visit(ty),
            HttpRequestBodyShape::Object { fields } => {
                for field in fields {
                    visit(&mut field.ty);
                }
            }
            HttpRequestBodyShape::Empty => {}
        }
        let response = &mut self.http.response;
        for binding in response.header.iter_mut().chain(&mut response.cookie) {
            visit(&mut binding.ty);
        }
        match &mut response.body.shape {
            HttpResponseBodyShape::ReturnOnly { ty }
            | HttpResponseBodyShape::SingleValue { ty, .. }
            | HttpResponseBodyShape::Stream { item_ty: ty, .. } => visit(ty),
            HttpResponseBodyShape::Object { fields } => {
                for field in fields {
                    visit(&mut field.ty);
                }
            }
            HttpResponseBodyShape::Empty => {}
        }
        for representation in &mut response.representations {
            visit(&mut representation.ty);
        }
    }
}
