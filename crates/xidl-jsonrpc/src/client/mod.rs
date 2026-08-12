#[cfg(test)]
mod test;

use crate::codec::Codec;
use crate::transport::Stream;
use crate::{Error, ErrorCode, JSONRPC_VERSION, RpcRequest, RpcResponse};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::io::BufStream;

pub struct Client<S> {
    stream: BufStream<S>,
    next_id: u64,
    codec: Codec,
}

impl<S> Client<S>
where
    S: Stream + Unpin,
{
    /// Creates a client using newline-delimited JSON framing.
    pub fn new(stream: S) -> Self {
        Self::with_codec(stream, Codec::Json)
    }

    /// Creates a client using length-prefixed MessagePack framing.
    #[cfg(feature = "msgpack")]
    pub fn new_msgpack(stream: S) -> Self {
        Self::with_codec(stream, Codec::Msgpack)
    }

    fn with_codec(stream: S, codec: Codec) -> Self {
        Self {
            stream: BufStream::new(stream),
            next_id: 1,
            codec,
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
        self.codec.write(&mut self.stream, &request).await?;

        let Some(response) = self.codec.read::<_, RpcResponse>(&mut self.stream).await? else {
            return Err(Error::Protocol("no response"));
        };
        if response.id.as_ref() != Some(&Value::from(id)) {
            return Err(Error::Protocol("unexpected JSON-RPC id"));
        }
        if let Some(error) = response.error {
            return Err(Error::Rpc {
                code: ErrorCode::from(error.code),
                message: error.message,
                data: error.data,
            });
        }
        let result = response.result.unwrap_or(Value::Null);
        Ok(serde_json::from_value(result)?)
    }
}
