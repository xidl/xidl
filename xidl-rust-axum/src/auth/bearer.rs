use axum::http::HeaderMap;
use axum::http::HeaderValue;
use axum::http::header::{AUTHORIZATION, HeaderName};
use axum_extra::headers::{Error as HeaderError, Header};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;

/// Parsed HTTP Basic auth credentials.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BasicAuth {
    /// Username portion of the credential.
    pub username: String,
    /// Optional password portion of the credential.
    pub password: Option<String>,
}

/// Errors returned while parsing Basic auth credentials.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BasicAuthError {
    /// No `Authorization` header was present.
    Missing,
    /// The header existed but was malformed.
    Invalid,
}

/// Extracts Basic auth credentials from request headers.
pub fn extract_basic_auth(headers: &HeaderMap) -> Result<BasicAuth, BasicAuthError> {
    let value = headers.get(AUTHORIZATION).ok_or(BasicAuthError::Missing)?;
    let value = value.to_str().map_err(|_| BasicAuthError::Invalid)?;
    parse_basic_auth(value)
}

/// Parses an `Authorization: Basic ...` header value.
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

/// Serializable bearer token payload.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BearerAuth {
    /// Bearer token without the scheme prefix.
    pub token: String,
}

impl BearerAuth {
    /// Converts a decoded typed header into a serializable token wrapper.
    pub fn from_header(header: BearerHeader) -> Self {
        let token = header.token().to_string();
        let token = if token.is_empty() {
            String::default()
        } else {
            token
        };
        Self { token }
    }
}

/// Typed `Authorization: Bearer ...` header.
#[derive(Debug, Clone)]
pub struct BearerHeader {
    token: String,
}

impl BearerHeader {
    /// Returns the bearer token value.
    pub fn token(&self) -> &str {
        &self.token
    }
}

impl Header for BearerHeader {
    fn name() -> &'static HeaderName {
        &AUTHORIZATION
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, HeaderError>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        let value = values.next().ok_or_else(HeaderError::invalid)?;
        let value = value.to_str().map_err(|_| HeaderError::invalid())?;
        let value = value.trim();
        if value.is_empty() {
            return Err(HeaderError::invalid());
        }
        let (scheme, token) = value.split_once(' ').unwrap_or((value, ""));
        if !scheme.eq_ignore_ascii_case("Bearer") {
            return Err(HeaderError::invalid());
        }
        Ok(Self {
            token: token.trim_start().to_string(),
        })
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        let value = if self.token.is_empty() {
            "Bearer".to_string()
        } else {
            format!("Bearer {}", self.token)
        };
        if let Ok(header_value) = HeaderValue::from_str(&value) {
            values.extend(std::iter::once(header_value));
        }
    }
}
