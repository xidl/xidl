//! Implements [OpenApi Responses][responses].
//!
//! [responses]: https://spec.openapis.org/oas/latest.html#responses-object

mod ext;
#[cfg(test)]
#[path = "../response_openapi_extensions_tests.rs"]
mod openapi_extensions_tests;
#[cfg(test)]
#[path = "../response_tests.rs"]
mod tests;
mod types;

pub use self::{
    ext::ResponseExt,
    types::{Response, ResponseBuilder, Responses, ResponsesBuilder},
};
