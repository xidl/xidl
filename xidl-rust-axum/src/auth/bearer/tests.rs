use super::*;
use axum_extra::headers::Header;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;

#[test]
fn parse_basic_auth_handles_password_and_missing_password() {
    let with_password = format!("Basic {}", STANDARD.encode("alice:secret"));
    let parsed = parse_basic_auth(&with_password).unwrap();
    assert_eq!(parsed.username, "alice");
    assert_eq!(parsed.password.as_deref(), Some("secret"));

    let without_password = format!("Basic {}", STANDARD.encode("alice"));
    let parsed = parse_basic_auth(&without_password).unwrap();
    assert_eq!(parsed.username, "alice");
    assert_eq!(parsed.password, None);
}

#[test]
fn parse_basic_auth_rejects_invalid_values() {
    assert!(matches!(
        parse_basic_auth("Bearer token"),
        Err(BasicAuthError::Invalid)
    ));
    assert!(matches!(
        parse_basic_auth("Basic !!!"),
        Err(BasicAuthError::Invalid)
    ));
}

#[test]
fn extract_basic_auth_reports_missing_header() {
    assert!(matches!(
        extract_basic_auth(&HeaderMap::new()),
        Err(BasicAuthError::Missing)
    ));
}

#[test]
fn bearer_header_decode_accepts_case_insensitive_scheme() {
    let value = HeaderValue::from_static("bearer token-123");
    let header = BearerHeader::decode(&mut std::iter::once(&value)).unwrap();
    assert_eq!(header.token(), "token-123");
    assert_eq!(BearerAuth::from_header(header).token, "token-123");
}

#[test]
fn bearer_header_decode_rejects_wrong_scheme() {
    let value = HeaderValue::from_static("Basic token");
    assert!(BearerHeader::decode(&mut std::iter::once(&value)).is_err());
}

#[test]
fn bearer_header_encode_omits_space_for_empty_token() {
    let mut encoded = Vec::new();
    BearerHeader {
        token: String::new(),
    }
    .encode(&mut encoded);
    assert_eq!(encoded[0].to_str().unwrap(), "Bearer");
}
