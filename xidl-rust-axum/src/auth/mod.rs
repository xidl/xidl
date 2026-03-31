//! Authentication helpers used by generated handlers and clients.

/// HTTP Basic authentication helpers.
pub mod basic;
/// HTTP Bearer authentication helpers and typed headers.
pub mod bearer;

/// Re-exports for basic and bearer auth helper types.
pub use basic::{
    BasicAuth, BasicAuthError, extract_basic_auth, parse_basic_auth, unauthorized_response,
};
pub use bearer::{BearerAuth, BearerHeader};
