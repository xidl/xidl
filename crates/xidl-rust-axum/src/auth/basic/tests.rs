use super::*;
use axum::http::header::WWW_AUTHENTICATE;

#[test]
fn unauthorized_response_sanitizes_realm() {
    let response = unauthorized_response("prod\\realm\"\n");
    let header = response.headers().get(WWW_AUTHENTICATE).unwrap();
    assert_eq!(header.to_str().unwrap(), "Basic realm=\"prodrealm\"");
}

#[test]
fn unauthorized_response_uses_default_realm_when_sanitized_empty() {
    let response = unauthorized_response("\"\n\\");
    let header = response.headers().get(WWW_AUTHENTICATE).unwrap();
    assert_eq!(header.to_str().unwrap(), "Basic realm=\"xidl\"");
}
