use crate::{Error, ErrorCode, RpcRequest, RpcResponse, JSONRPC_VERSION};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};

pub struct Client<R, W> {
    reader: R,
    writer: W,
    next_id: u64,
}

impl<R, W> Client<R, W>
where
    R: AsyncBufRead + Unpin,
    W: AsyncWrite + Unpin,
{
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader,
            writer,
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
        let payload = serde_json::to_string(&request)?;
        self.writer.write_all(payload.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.writer.flush().await?;

        let mut line = String::new();
        let bytes = self.reader.read_line(&mut line).await?;
        if bytes == 0 {
            return Err(Error::Protocol("no response"));
        }

        let response: RpcResponse = serde_json::from_str(&line)?;
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
