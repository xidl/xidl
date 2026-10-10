//! Authentication helpers used by generated handlers and clients.

/// API key authentication helpers.
pub mod api_key;
/// HTTP Basic authentication helpers.
pub mod basic;
/// HTTP Bearer authentication helpers and typed headers.
pub mod bearer;

/// Re-exports for API key auth helper types.
pub use api_key::{ApiKeyAuth, ApiKeyAuthError, ApiKeyLocation, extract_api_key};
#[cfg(feature = "router")]
pub use basic::unauthorized_response;
/// Re-exports for basic and bearer auth helper types.
pub use basic::{BasicAuth, BasicAuthError, extract_basic_auth, parse_basic_auth};
pub use bearer::{BearerAuth, BearerHeader};

mod credentials;
/// Credentials shared by generated clients and authenticated handlers.
pub use credentials::ClientAuth;
