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
