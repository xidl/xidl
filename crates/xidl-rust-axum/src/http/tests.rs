use super::serde_ext::default_on_missing_reject_null;
use super::*;
use http::{HeaderMap, HeaderValue, header};
use serde::Deserialize;

#[test]
fn shared_content_types_compose_with_runtime_codecs_and_headers() {
    let shared: xidl_http::ContentType = ContentType::Json;
    let json = crate::SerializeFactory::new(shared.as_str())
        .to_vec(&vec![1, 2])
        .expect("shared JSON representation");
    let decoded: Vec<u8> = crate::DeserializeFactory::new(ContentType::Json.as_str())
        .from_slice(&json)
        .expect("decode shared JSON representation");
    assert_eq!(decoded, [1, 2]);
    let text = crate::SerializeFactory::new(ContentType::Text.as_str())
        .to_vec(&"plain body")
        .expect("shared text representation");
    assert_eq!(text, b"plain body");
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("Application/Octet-Stream; mode=raw"),
    );
    assert!(content_type_matches(
        &headers,
        ContentType::OctetStream.as_str()
    ));
    headers.insert(header::ACCEPT, HeaderValue::from_static("TEXT/*"));
    assert!(accepts_media_type(&headers, ContentType::Text.as_str()));
    assert!(!accepts_media_type(&headers, ContentType::Json.as_str()));
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct DefaultedPayload {
    #[serde(
        default = "default_count",
        deserialize_with = "default_on_missing_reject_null"
    )]
    count: u32,
}

fn default_count() -> u32 {
    7
}

#[test]
fn accepts_any_when_header_is_missing() {
    assert!(accepts_media_type(&HeaderMap::new(), "application/json"));
}

#[test]
fn accepts_exact_media_type_and_type_wildcard() {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::ACCEPT,
        HeaderValue::from_static("text/*, application/xml"),
    );
    assert!(accepts_media_type(&headers, "text/plain"));
    assert!(!accepts_media_type(&headers, "application/json"));
}

#[test]
fn accepts_ignores_invalid_accept_values_and_honors_wildcard() {
    let mut headers = HeaderMap::new();
    headers.append(header::ACCEPT, HeaderValue::from_bytes(b"\xff").unwrap());
    headers.append(header::ACCEPT, HeaderValue::from_static("*/*"));
    assert!(accepts_media_type(&headers, "application/json"));
}

#[test]
fn content_type_matches_ignores_parameters() {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json; charset=utf-8"),
    );
    assert!(content_type_matches(&headers, "application/json"));
    assert!(!content_type_matches(&headers, "application/xml"));
}

#[test]
fn media_type_helpers_reject_invalid_values() {
    assert!(media_type_eq(
        "Application/Json; charset=utf-8",
        "application/json"
    ));
    assert!(!media_type_eq("application-json", "application/json"));
    assert_eq!(
        canonical_media_type("application/json; charset=utf-8"),
        Some("application/json")
    );
    assert_eq!(canonical_media_type("invalid"), None);
}

#[test]
fn serde_ext_uses_default_on_missing_and_rejects_null() {
    let missing: DefaultedPayload = serde_json::from_str("{}").unwrap();
    assert_eq!(missing, DefaultedPayload { count: 7 });

    let err = serde_json::from_str::<DefaultedPayload>(r#"{"count":null}"#).unwrap_err();
    assert!(err.to_string().contains("null is not allowed"));
}

#[test]
fn response_cookies_decode_escaped_values_without_attributes() {
    let mut headers = HeaderMap::new();
    headers.append(
        header::SET_COOKIE,
        HeaderValue::from_static("session=001%2F%3B%25; HttpOnly; Path=/"),
    );
    headers.append(
        header::SET_COOKIE,
        HeaderValue::from_static("session=next%20value"),
    );
    let cookies = parse_set_cookies(headers.get_all(header::SET_COOKIE).iter());
    assert_eq!(cookies["session"], ["001/;%", "next value"]);
}
