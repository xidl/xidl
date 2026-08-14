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
pub use client::Client;
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
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<RpcError>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct RpcError {
    code: i64,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}
