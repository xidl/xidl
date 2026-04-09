//! Implements OpenAPI security schema types.

#[path = "security_core.rs"]
mod security_core;
#[path = "security_flow.rs"]
mod security_flow;
#[path = "security_http.rs"]
mod security_http;
#[path = "security_oauth.rs"]
mod security_oauth;
#[path = "security_scopes.rs"]
mod security_scopes;
#[cfg(test)]
#[path = "security_tests.rs"]
mod security_tests;
#[cfg(test)]
#[path = "security_tests_oauth.rs"]
mod security_tests_oauth;

pub use self::security_core::{ApiKey, ApiKeyValue, SecurityRequirement, SecurityScheme};
pub use self::security_flow::{AuthorizationCode, ClientCredentials, Implicit, Password};
pub use self::security_http::{Http, HttpAuthScheme, HttpBuilder, OpenIdConnect};
pub use self::security_oauth::{Flow, OAuth2};
pub use self::security_scopes::Scopes;
