//! Implements [OpenAPI Server Object][server] types to configure target servers.
//!
//! [server]: https://spec.openapis.org/oas/latest.html#server-object

mod server_object;
#[cfg(test)]
mod tests;
mod variable;

pub use self::{
    server_object::{Server, ServerBuilder},
    variable::{ServerVariable, ServerVariableBuilder},
};
