pub use crate::auth::bearer::{BasicAuth, BasicAuthError, extract_basic_auth, parse_basic_auth};
use axum::http::HeaderValue;
use axum::response::IntoResponse;

/// Builds a `401 Unauthorized` response with a Basic auth challenge.
pub fn unauthorized_response(realm: &str) -> axum::response::Response {
    let mut resp = crate::Error::unauthorized().into_response();
    let realm = sanitize_realm(realm);
    let header_value = format!("Basic realm=\"{}\"", realm);
    if let Ok(value) = HeaderValue::from_str(&header_value) {
        resp.headers_mut()
            .insert(axum::http::header::WWW_AUTHENTICATE, value);
    }
    resp
}

fn sanitize_realm(realm: &str) -> String {
    let mut out = String::new();
    for ch in realm.chars() {
        if ch == '"' || ch == '\\' || ch.is_control() {
            continue;
        }
        out.push(ch);
    }
    if out.is_empty() {
        "xidl".to_string()
    } else {
        out
    }
}
