use serde::{Deserialize, Serialize};
use serde_json::Value;

#[cfg(feature = "tokio")]
mod client;
#[cfg(feature = "tokio")]
mod codec;
mod error;
#[cfg(feature = "tokio")]
mod rpc;
#[cfg(feature = "tokio")]
mod server;
#[cfg(feature = "tokio")]
pub mod stream;

#[cfg(feature = "tokio")]
pub use client::{Client, ConcurrentClient};
pub use error::{Error, ErrorCode};
#[cfg(feature = "tokio")]
pub use rpc::RpcClient;
#[cfg(feature = "tokio")]
pub use server::{Handler, Server, ServerBuilder};
#[cfg(feature = "tokio")]
pub mod transport;
pub use futures_util;
#[cfg(feature = "transport-tcp")]
pub use transport::TcpListener;
#[cfg(feature = "tokio")]
pub use transport::{BoundListener, Listener, Stream, bind, connect, connect_inproc};
#[cfg(feature = "tokio")]
pub use transport::{InprocListener, IoListener};
#[cfg(all(feature = "transport-ipc", unix))]
pub use transport::{IpcListener, connect_ipc};
#[cfg(feature = "transport-quic")]
pub use transport::{QuicListener, connect_quic};
#[cfg(feature = "transport-tls")]
pub use transport::{TlsListener, connect_tls};
#[cfg(feature = "transport-websocket")]
pub use transport::{WebSocketListener, connect_websocket};

const JSONRPC_VERSION: &str = "2.0";

/// Payload representation used by message-oriented transports.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameKind {
    /// UTF-8 text, used by JSON transports.
    Text,
    /// Opaque bytes, used by MessagePack transports.
    Binary,
}

#[derive(Serialize)]
pub(crate) struct RpcRequest<'a, P> {
    jsonrpc: &'static str,
    id: u64,
    method: &'a str,
    params: P,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct RpcResponse {
    jsonrpc: Option<String>,
    id: Option<Value>,
    #[serde(default, deserialize_with = "deserialize_present_value")]
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<RpcError>,
}

fn deserialize_present_value<'de, D>(deserializer: D) -> Result<Option<Value>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Value::deserialize(deserializer).map(Some)
}

impl RpcResponse {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        if self.jsonrpc.as_deref() != Some(JSONRPC_VERSION) {
            return Err(Error::Protocol("invalid JSON-RPC response version"));
        }
        match (self.result.is_some(), self.error.is_some()) {
            (true, false) | (false, true) => Ok(()),
            _ => Err(Error::Protocol(
                "JSON-RPC response must contain exactly one result or error",
            )),
        }
    }

    pub(crate) fn into_result(self) -> Result<Value, Error> {
        self.validate()?;
        if let Some(error) = self.error {
            return Err(Error::Rpc {
                code: ErrorCode::from(error.code),
                message: error.message,
                data: error.data,
            });
        }
        Ok(self.result.unwrap_or(Value::Null))
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct RpcError {
    code: i64,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}
