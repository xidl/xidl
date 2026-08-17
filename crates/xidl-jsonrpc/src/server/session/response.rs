use crate::{Error, ErrorCode, JSONRPC_VERSION, RpcError, RpcResponse};
use serde_json::Value;

pub(super) struct ResponseCodec;

impl ResponseCodec {
    pub(super) fn success(id: Option<Value>, result: Value) -> RpcResponse {
        RpcResponse {
            jsonrpc: Some(JSONRPC_VERSION.to_string()),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub(super) fn error(id: Option<Value>, error: Error) -> RpcResponse {
        RpcResponse {
            jsonrpc: Some(JSONRPC_VERSION.to_string()),
            id,
            result: None,
            error: Some(Self::rpc_error(error)),
        }
    }

    pub(super) fn rpc_error(error: Error) -> RpcError {
        match error {
            Error::Rpc {
                code,
                message,
                data,
            } => RpcError {
                code: code.code(),
                message,
                data,
            },
            Error::Json(err) => RpcError {
                code: ErrorCode::ParseError.code(),
                message: err.to_string(),
                data: None,
            },
            #[cfg(feature = "msgpack")]
            Error::Msgpack(message) => RpcError {
                code: ErrorCode::ParseError.code(),
                message,
                data: None,
            },
            Error::FrameTooLarge { .. } => RpcError {
                code: ErrorCode::ParseError.code(),
                message: error.to_string(),
                data: None,
            },
            Error::Protocol(message) => RpcError {
                code: ErrorCode::InvalidRequest.code(),
                message: message.to_string(),
                data: None,
            },
            Error::RequestTimeout => RpcError {
                code: ErrorCode::ServerError.code(),
                message: "internal server error".to_string(),
                data: None,
            },
            Error::Io(_) => RpcError {
                code: ErrorCode::InternalError.code(),
                message: "internal server error".to_string(),
                data: None,
            },
        }
    }
}
