use super::*;
use axum::http::{HeaderMap, HeaderValue, header};

#[test]
fn request_preserves_headers_and_payload() {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );

    let request = Request::new(headers.clone(), "payload".to_string());

    assert_eq!(
        request.headers().get(header::CONTENT_TYPE),
        headers.get(header::CONTENT_TYPE)
    );
    assert_eq!(request.clone().into_inner(), "payload");
    assert_eq!(request.data, "payload");
}
