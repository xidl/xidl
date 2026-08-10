use super::{ApiKeyAuthError, ApiKeyLocation, extract_api_key};
use axum::http::{HeaderMap, HeaderValue, Uri, header};

fn headers_with_cookie(value: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(header::COOKIE, value.parse().unwrap());
    headers
}

#[test]
fn extract_header_key_returns_credential() {
    let mut headers = HeaderMap::new();
    headers.insert("X-Org-Key", "secret-123".parse().unwrap());
    let auth = extract_api_key(
        &headers,
        &Uri::from_static("/"),
        ApiKeyLocation::Header,
        "X-Org-Key",
    )
    .unwrap();
    assert_eq!(auth.location, ApiKeyLocation::Header);
    assert_eq!(auth.name, "X-Org-Key");
    assert_eq!(auth.value, "secret-123");
}

#[test]
fn extract_header_key_reports_missing() {
    let result = extract_api_key(
        &HeaderMap::new(),
        &Uri::from_static("/"),
        ApiKeyLocation::Header,
        "X-Org-Key",
    );
    assert_eq!(result, Err(ApiKeyAuthError::Missing));
}

#[test]
fn extract_header_key_reports_invalid_utf8() {
    let mut headers = HeaderMap::new();
    headers.insert("X-Org-Key", HeaderValue::from_bytes(b"\xff\xfe").unwrap());
    let result = extract_api_key(
        &headers,
        &Uri::from_static("/"),
        ApiKeyLocation::Header,
        "X-Org-Key",
    );
    assert_eq!(result, Err(ApiKeyAuthError::InvalidHeaderValue));
}

#[test]
fn extract_query_key_returns_credential() {
    let uri = Uri::from_static("/stops?org_key=abc&page=2");
    let auth = extract_api_key(&HeaderMap::new(), &uri, ApiKeyLocation::Query, "org_key").unwrap();
    assert_eq!(auth.location, ApiKeyLocation::Query);
    assert_eq!(auth.name, "org_key");
    assert_eq!(auth.value, "abc");
}

#[test]
fn extract_query_key_reports_missing() {
    let uri = Uri::from_static("/stops?page=2");
    let result = extract_api_key(&HeaderMap::new(), &uri, ApiKeyLocation::Query, "org_key");
    assert_eq!(result, Err(ApiKeyAuthError::Missing));
}

#[test]
fn extract_query_key_reports_missing_without_query_string() {
    let uri = Uri::from_static("/stops");
    let result = extract_api_key(&HeaderMap::new(), &uri, ApiKeyLocation::Query, "org_key");
    assert_eq!(result, Err(ApiKeyAuthError::Missing));
}

#[test]
fn extract_cookie_key_returns_credential() {
    let headers = headers_with_cookie("session=abc; org_key=xyz; theme=dark");
    let auth = extract_api_key(
        &headers,
        &Uri::from_static("/"),
        ApiKeyLocation::Cookie,
        "org_key",
    )
    .unwrap();
    assert_eq!(auth.location, ApiKeyLocation::Cookie);
    assert_eq!(auth.name, "org_key");
    assert_eq!(auth.value, "xyz");
}

#[test]
fn extract_cookie_key_accepts_empty_value() {
    let headers = headers_with_cookie("org_key=; theme=dark");
    let auth = extract_api_key(
        &headers,
        &Uri::from_static("/"),
        ApiKeyLocation::Cookie,
        "org_key",
    )
    .unwrap();
    assert_eq!(auth.value, "");
}

#[test]
fn extract_cookie_key_reports_missing() {
    let headers = headers_with_cookie("theme=dark");
    let result = extract_api_key(
        &headers,
        &Uri::from_static("/"),
        ApiKeyLocation::Cookie,
        "org_key",
    );
    assert_eq!(result, Err(ApiKeyAuthError::Missing));
}

#[test]
fn extract_cookie_key_reports_missing_without_cookie_header() {
    let result = extract_api_key(
        &HeaderMap::new(),
        &Uri::from_static("/"),
        ApiKeyLocation::Cookie,
        "org_key",
    );
    assert_eq!(result, Err(ApiKeyAuthError::Missing));
}
