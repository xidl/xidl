use axum::http::header::AUTHORIZATION;
use axum::http::{HeaderMap, HeaderValue};
use axum::response::IntoResponse;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BasicAuth {
    pub username: String,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BasicAuthError {
    Missing,
    Invalid,
}

pub fn extract_basic_auth(headers: &HeaderMap) -> Result<BasicAuth, BasicAuthError> {
    let value = headers.get(AUTHORIZATION).ok_or(BasicAuthError::Missing)?;
    let value = value.to_str().map_err(|_| BasicAuthError::Invalid)?;
    parse_basic_auth(value)
}

pub fn parse_basic_auth(value: &str) -> Result<BasicAuth, BasicAuthError> {
    let value = value.trim();
    let (scheme, token) = value.split_once(' ').ok_or(BasicAuthError::Invalid)?;
    if !scheme.eq_ignore_ascii_case("Basic") {
        return Err(BasicAuthError::Invalid);
    }
    if token.is_empty() {
        return Err(BasicAuthError::Invalid);
    }
    let decoded = STANDARD
        .decode(token.as_bytes())
        .map_err(|_| BasicAuthError::Invalid)?;
    let decoded = String::from_utf8(decoded).map_err(|_| BasicAuthError::Invalid)?;
    let (username, password) = match decoded.split_once(':') {
        Some((user, pass)) => (user.to_string(), Some(pass.to_string())),
        None => (decoded, None),
    };
    Ok(BasicAuth { username, password })
}

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
