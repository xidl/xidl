use serde::{Deserialize, Serialize};
use serde_json::Value;

#[cfg(feature = "tokio")]
mod client;
mod error;
#[cfg(feature = "tokio")]
mod line_io;
#[cfg(feature = "tokio")]
mod server;
pub mod stream;

#[cfg(feature = "tokio")]
pub use client::Client;
pub use error::{Error, ErrorCode};
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
#[cfg(all(feature = "transport-quic", not(tarpaulin_include)))]
pub use transport::{QuicListener, connect_quic};
#[cfg(all(feature = "transport-tls", not(tarpaulin_include)))]
pub use transport::{TlsListener, connect_tls};
#[cfg(all(feature = "transport-websocket", not(tarpaulin_include)))]
pub use transport::{WebSocketListener, connect_websocket};

const JSONRPC_VERSION: &str = "2.0";

#[derive(Serialize)]
pub(crate) struct RpcRequest<'a, P> {
    jsonrpc: &'static str,
    id: u64,
    method: &'a str,
    params: P,
}

#[derive(Deserialize)]
pub(crate) struct RpcRequestOwned {
    id: Option<u64>,
    method: Option<String>,
    params: Option<Value>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct RpcResponse {
    jsonrpc: Option<String>,
    id: Option<u64>,
    result: Option<Value>,
    error: Option<RpcError>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct RpcError {
    code: i64,
    message: String,
    data: Option<Value>,
}
