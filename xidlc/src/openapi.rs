//! Rust implementation of Openapi Spec V3.1.

mod document;
mod macros;
mod shared;
#[cfg(test)]
mod tests;

pub use self::{
    content::{Content, ContentBuilder},
    document::{OpenApi, OpenApiBuilder},
    external_docs::ExternalDocs,
    header::{Header, HeaderBuilder},
    info::{Contact, ContactBuilder, Info, InfoBuilder, License, LicenseBuilder},
    path::{HttpMethod, PathItem, Paths, PathsBuilder},
    response::{Response, ResponseBuilder, Responses, ResponsesBuilder},
    schema::{
        AllOf, AllOfBuilder, Array, ArrayBuilder, Components, ComponentsBuilder, Discriminator,
        KnownFormat, Object, ObjectBuilder, OneOf, OneOfBuilder, Ref, Schema, SchemaFormat,
        ToArray, Type,
    },
    security::SecurityRequirement,
    server::{Server, ServerBuilder, ServerVariable, ServerVariableBuilder},
    shared::{Deprecated, Number, OpenApiVersion, RefOr, Required},
    tag::Tag,
};

pub mod content;
pub mod encoding;
pub mod example;
pub mod extensions;
pub mod external_docs;
pub mod header;
pub mod info;
pub mod link;
pub mod path;
pub mod request_body;
pub mod response;
pub mod schema;
pub mod security;
pub mod server;
pub mod tag;
pub mod xml;

pub(crate) use macros::{build_fn, builder, from, new, set_value};
