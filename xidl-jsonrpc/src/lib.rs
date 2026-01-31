use serde::{Deserialize, Serialize};
use serde_json::Value;

mod client;
mod error;
mod server;

pub use client::Client;
pub use error::{Error, ErrorCode};
pub use server::{serve, Handler, Io, Server, ServerBuilder};

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
