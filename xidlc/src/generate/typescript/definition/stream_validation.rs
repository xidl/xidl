use crate::error::{IdlcError, IdlcResult};
use xidl_parser::rest_hir::semantics::{
    HttpStreamCodec, HttpStreamConfig, HttpStreamKind, HttpStreamTargetSupport,
    validate_http_stream_method, validate_http_stream_target,
};

use super::operation::http_method_name;
use xidl_parser::rest_hir::HttpMethod;

pub(crate) fn validate_target(op_ident: &str, stream: HttpStreamConfig) -> IdlcResult<()> {
    validate_http_stream_target(op_ident, stream, target_support()).map_err(IdlcError::rpc)
}

pub(crate) fn validate_method(
    op_ident: &str,
    stream: Option<HttpStreamKind>,
    method: HttpMethod,
) -> IdlcResult<()> {
    validate_http_stream_method(op_ident, stream, http_method_name(method), target_support())
        .map_err(IdlcError::rpc)
}

fn target_support() -> HttpStreamTargetSupport<'static> {
    HttpStreamTargetSupport {
        target: "typescript",
        supports_bidi: false,
        server_codec: HttpStreamCodec::Sse,
        client_codec: HttpStreamCodec::Ndjson,
        server_method: "GET",
        client_method: "POST",
        bidi_method: "GET",
    }
}
