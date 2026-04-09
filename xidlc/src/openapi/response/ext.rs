use super::{Response, ResponseBuilder};
use crate::openapi::Content;

/// Trait with convenience functions for documenting response bodies.
pub trait ResponseExt {
    /// Add [`Content`] to [`Response`] referring to a _`schema`_
    /// with Content-Type `application/json`.
    fn json_schema_ref(self, ref_name: &str) -> Self;
}

impl ResponseExt for Response {
    fn json_schema_ref(mut self, ref_name: &str) -> Response {
        self.content.insert(
            "application/json".to_string(),
            Content::new(Some(crate::openapi::Ref::from_schema_name(ref_name))),
        );
        self
    }
}

impl ResponseExt for ResponseBuilder {
    fn json_schema_ref(self, ref_name: &str) -> ResponseBuilder {
        self.content(
            "application/json",
            Content::new(Some(crate::openapi::Ref::from_schema_name(ref_name))),
        )
    }
}
