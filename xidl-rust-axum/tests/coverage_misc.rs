use std::borrow::Cow;

use axum::http::StatusCode;
use serde::Serialize;
use xidl_rust_axum::http::{accepts_media_type, content_type_matches};
use xidl_rust_axum::{DeserializeFactory, Error, ErrorBody, SerializeFactory};

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
struct Payload {
    value: u32,
}

struct FailingSerialize;

impl Serialize for FailingSerialize {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Err(serde::ser::Error::custom("boom"))
    }
}

#[test]
fn error_helpers_cover_message_status_and_body_paths() {
    let err = Error::message("broken");
    assert_eq!(err.http_status(), StatusCode::INTERNAL_SERVER_ERROR);

    let err = Error::from_http_response(
        StatusCode::BAD_REQUEST,
        Some(ErrorBody {
            code: 418,
            msg: Cow::Borrowed("teapot"),
        }),
    );
    assert_eq!(err.code, 418);
    assert_eq!(err.message, "teapot");

    let ok = Error::ok();
    assert_eq!(ok.http_status(), StatusCode::OK);
}

#[test]
fn serde_factories_cover_mime_getters_and_failure_paths() {
    let serializer = SerializeFactory::new("APPLICATION/JSON");
    assert_eq!(serializer.mime(), "APPLICATION/JSON");
    let err = serializer.to_vec(&FailingSerialize).unwrap_err();
    assert_eq!(err.code, 500);

    let deserializer = DeserializeFactory::new("APPLICATION/X-WWW-FORM-URLENCODED");
    assert_eq!(deserializer.mime(), "APPLICATION/X-WWW-FORM-URLENCODED");
    let err = deserializer
        .from_slice::<Payload>(b"not=valid")
        .unwrap_err();
    assert_eq!(err.code, 400);

    let err = DeserializeFactory::new("application/json")
        .from_slice::<Payload>(br#"{"value":"bad"}"#)
        .unwrap_err();
    assert_eq!(err.code, 400);
}

#[test]
fn serde_factories_reject_unsupported_mime_types() {
    let serializer = std::panic::catch_unwind(|| SerializeFactory::new("text/plain"));
    assert!(serializer.is_err());

    let deserializer = std::panic::catch_unwind(|| DeserializeFactory::new("text/plain"));
    assert!(deserializer.is_err());
}

#[test]
fn http_helpers_cover_invalid_expected_and_invalid_content_type() {
    let headers = axum::http::HeaderMap::new();
    assert!(!accepts_media_type(&headers, "invalid"));
    assert!(!accepts_media_type(&headers, "applicationjson"));

    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_bytes(b"\xff").unwrap(),
    );
    assert!(!content_type_matches(&headers, "application/json"));
    assert!(!content_type_matches(
        &axum::http::HeaderMap::new(),
        "application/json"
    ));
}
