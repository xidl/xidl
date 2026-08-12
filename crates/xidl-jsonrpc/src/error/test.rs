use super::{Error, ErrorCode};

#[test]
fn error_code_values_and_display_match_jsonrpc_constants() {
    let pairs = [
        (ErrorCode::ParseError, -32700),
        (ErrorCode::InvalidRequest, -32600),
        (ErrorCode::MethodNotFound, -32601),
        (ErrorCode::InvalidParams, -32602),
        (ErrorCode::InternalError, -32603),
        (ErrorCode::ServerError, -32000),
    ];

    for (code, value) in pairs {
        assert_eq!(code.code(), value);
        assert_eq!(code.to_string(), value.to_string());
    }
}

#[test]
fn helper_constructors_build_expected_rpc_errors() {
    let missing = Error::method_not_found("ping");
    assert!(missing.is_method_not_found());
    assert!(matches!(
        missing,
        Error::Rpc {
            code: ErrorCode::MethodNotFound,
            ref message,
            data: None,
        } if message == "method not found: ping"
    ));

    let invalid = Error::invalid_params("bad payload");
    assert!(!invalid.is_method_not_found());
    assert!(matches!(
        invalid,
        Error::Rpc {
            code: ErrorCode::InvalidParams,
            ref message,
            data: None,
        } if message == "bad payload"
    ));

    assert!(!Error::Protocol("oops").is_method_not_found());
}

#[test]
fn error_code_from_i64_preserves_reserved_codes_and_maps_rest_to_custom() {
    for raw in [-32700, -32600, -32601, -32602, -32603, -32000] {
        assert_eq!(ErrorCode::from(raw).code(), raw);
    }

    let custom = ErrorCode::from(-32001);
    assert!(matches!(custom, ErrorCode::Custom(code) if code == -32001));
    assert_eq!(custom.code(), -32001);
    assert_eq!(custom.to_string(), "-32001");
}
