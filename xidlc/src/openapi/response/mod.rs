//! Implements [OpenApi Responses][responses].
//!
//! [responses]: https://spec.openapis.org/oas/latest.html#responses-object

mod ext;
#[cfg(test)]
mod openapi_extensions_tests;
#[cfg(test)]
mod tests;
mod types;

pub use self::{
    ext::ResponseExt,
    types::{Response, ResponseBuilder, Responses, ResponsesBuilder},
};
