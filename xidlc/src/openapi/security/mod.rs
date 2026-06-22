//! Implements OpenAPI security schema types.

mod security_core;
mod security_flow;
mod security_http;
mod security_oauth;
mod security_scopes;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_oauth;

pub use self::security_core::{ApiKey, ApiKeyValue, SecurityRequirement, SecurityScheme};
pub use self::security_flow::{AuthorizationCode, ClientCredentials, Implicit, Password};
pub use self::security_http::{Http, HttpAuthScheme, HttpBuilder, OpenIdConnect};
pub use self::security_oauth::{Flow, OAuth2};
pub use self::security_scopes::Scopes;
