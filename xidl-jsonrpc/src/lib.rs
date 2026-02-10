use std::net::SocketAddr;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[cfg(feature = "tokio")]
mod client;
mod error;
#[cfg(feature = "tokio")]
mod server;

#[cfg(feature = "tokio")]
pub use client::Client;
pub use error::{Error, ErrorCode};
#[cfg(feature = "tokio")]
pub use server::{Handler, Io, Server, ServerBuilder};

mod stream;
pub use stream::AsyncStream;

const JSONRPC_VERSION: &str = "2.0";

#[async_trait::async_trait]
pub trait Listener: Send {
    async fn accept(
        &self,
    ) -> std::io::Result<(Box<dyn AsyncStream + Unpin + Send + 'static>, SocketAddr)>;
}

#[cfg(feature = "tokio-net")]
#[async_trait::async_trait]
impl Listener for tokio::net::TcpListener {
    async fn accept(
        &self,
    ) -> std::io::Result<(Box<dyn AsyncStream + Unpin + Send + 'static>, SocketAddr)> {
        let (stream, peer) = tokio::net::TcpListener::accept(self).await?;
        Ok((Box::new(stream), peer))
    }
}

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
