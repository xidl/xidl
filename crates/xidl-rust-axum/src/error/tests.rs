use super::*;

#[cfg(feature = "router")]
#[test]
fn invalid_error_code_falls_back_to_internal_server_error() {
    let response = Error::new(42, "boom").into_response();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[test]
fn http_response_fallback_preserves_http_status_code() {
    let err = Error::from_http_response(StatusCode::NOT_FOUND, None);
    assert_eq!(err.code, 404);
}
