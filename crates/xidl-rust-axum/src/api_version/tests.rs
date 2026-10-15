use http::{HeaderMap, Uri};

use super::{ApiVersionLocation, UnsupportedApiVersion, resolve_strict};

fn uri(query: &str) -> Uri {
    Uri::builder()
        .path_and_query(format!("/pets?{query}"))
        .build()
        .expect("valid uri")
}

#[test]
fn header_location_reads_the_declared_header() {
    let mut headers = HeaderMap::new();
    headers.insert("Api-Version", "2025-01".parse().expect("header value"));
    let raw = ApiVersionLocation::Header.extract(&headers, &uri("x=1"), "Api-Version");
    assert_eq!(raw.as_deref(), Some("2025-01"));
}

#[test]
fn query_location_decodes_percent_escapes() {
    let raw = ApiVersionLocation::Query.extract(
        &HeaderMap::new(),
        &uri("a=1&api-version=2025%2D01"),
        "api-version",
    );
    assert_eq!(raw.as_deref(), Some("2025-01"));
}

#[test]
fn missing_selector_extracts_to_none() {
    let raw = ApiVersionLocation::Header.extract(
        &HeaderMap::new(),
        &uri("api-version=2025-01"),
        "X-Version",
    );
    assert_eq!(raw, None);
}

#[test]
fn strict_policy_accepts_only_the_supported_value() {
    assert_eq!(resolve_strict(Some("2025-01"), "2025-01"), Ok(()));
    assert_eq!(
        resolve_strict(Some("2024-12"), "2025-01"),
        Err(UnsupportedApiVersion {
            requested: Some("2024-12".to_string()),
            supported: "2025-01".to_string(),
        })
    );
    assert_eq!(
        resolve_strict(None, "2025-01"),
        Err(UnsupportedApiVersion {
            requested: None,
            supported: "2025-01".to_string(),
        })
    );
}

#[test]
fn unsupported_version_maps_to_a_typed_400() {
    let error: crate::Error = UnsupportedApiVersion {
        requested: Some("2024-12".to_string()),
        supported: "2025-01".to_string(),
    }
    .into();
    assert_eq!(error.code, 400);
    assert!(error.message.contains("2024-12"));
    assert!(error.message.contains("2025-01"));
}
