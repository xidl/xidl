#[cfg(test)]
mod test;

use crate::line_io::{read_json_line, write_json_line};
use crate::transport::Stream;
use crate::{Error, ErrorCode, JSONRPC_VERSION, RpcRequest, RpcResponse};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::io::BufStream;

pub struct Client<S> {
    stream: BufStream<S>,
    next_id: u64,
}

impl<S> Client<S>
where
    S: Stream + Unpin,
{
    pub fn new(stream: S) -> Self {
        Self {
            stream: BufStream::new(stream),
            next_id: 1,
        }
    }

    pub async fn call<P, T>(&mut self, method: &str, params: P) -> Result<T, Error>
    where
        P: Serialize,
        T: DeserializeOwned,
    {
        let id = self.next_id;
        self.next_id += 1;

        let request = RpcRequest {
            jsonrpc: JSONRPC_VERSION,
            id,
            method,
            params,
        };
        write_json_line(&mut self.stream, &request).await?;

        let Some(response) = read_json_line::<_, RpcResponse>(&mut self.stream).await? else {
            return Err(Error::Protocol("no response"));
        };
        if response.id != Some(id) {
            return Err(Error::Protocol("unexpected JSON-RPC id"));
        }
        if let Some(error) = response.error {
            return Err(Error::Rpc {
                code: ErrorCode::ServerError,
                message: error.message,
                data: error.data,
            });
        }
        let result = response.result.unwrap_or(Value::Null);
        Ok(serde_json::from_value(result)?)
    }
}
