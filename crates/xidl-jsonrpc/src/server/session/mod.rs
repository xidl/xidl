use crate::codec::Codec;
use crate::{Error, ErrorCode, Handler, JSONRPC_VERSION, RpcError, RpcRequestOwned, RpcResponse};
use serde_json::Value;
use tokio::io::{AsyncRead, AsyncWrite, BufStream};

pub(crate) struct ServerSession<RW, H> {
    stream: Option<BufStream<RW>>,
    handler: H,
    codec: Codec,
}

impl<RW, H> ServerSession<RW, H>
where
    H: Handler,
    RW: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    pub(crate) fn with_codec(stream: RW, handler: H, codec: Codec) -> Self {
        let stream = tokio::io::BufStream::new(stream);
        Self {
            stream: Some(stream),
            handler,
            codec,
        }
    }

    pub(crate) async fn run(&mut self) -> Result<(), Error> {
        loop {
            let Some(stream) = self.stream.as_mut() else {
                break;
            };
            let request = match self.codec.read::<_, RpcRequestOwned>(stream).await {
                Ok(Some(request)) => request,
                Ok(None) => break,
                Err(err) if Self::is_decode_error(&err) => {
                    self.write_error(None, err).await?;
                    continue;
                }
                Err(err) => return Err(err),
            };
            if !self.handle_request(request).await? {
                break;
            }
        }
        Ok(())
    }

    fn is_decode_error(error: &Error) -> bool {
        match error {
            Error::Json(_) => true,
            #[cfg(feature = "msgpack")]
            Error::Msgpack(_) => true,
            _ => false,
        }
    }

    async fn handle_request(&mut self, request: RpcRequestOwned) -> Result<bool, Error> {
        let id = request.id;
        let method = match request.method {
            Some(method) => method,
            None => {
                self.write_error(id, Error::Protocol("missing method"))
                    .await?;
                return Ok(true);
            }
        };
        let params = request.params.unwrap_or(Value::Null);

        if self.handler.accepts_bidi(&method) {
            let stream = self
                .stream
                .take()
                .ok_or(Error::Protocol("missing stream"))?;
            let bidi = crate::stream::open_bidi_server_with(stream, self.codec);
            self.handler.handle_bidi(&method, params, bidi).await?;
            return Ok(false);
        }

        match self.handler.handle(&method, params).await {
            Ok(value) => self.write_result(id, value).await?,
            Err(err) => self.write_error(id, err).await?,
        }
        Ok(true)
    }

    async fn write_result(&mut self, id: Option<u64>, result: Value) -> Result<(), Error> {
        self.write_response(Self::success_response(id, result))
            .await
    }

    async fn write_error(&mut self, id: Option<u64>, error: Error) -> Result<(), Error> {
        self.write_response(Self::error_response(id, error)).await
    }

    fn success_response(id: Option<u64>, result: Value) -> RpcResponse {
        RpcResponse {
            jsonrpc: Some(JSONRPC_VERSION.to_string()),
            id,
            result: Some(result),
            error: None,
        }
    }

    fn error_response(id: Option<u64>, error: Error) -> RpcResponse {
        RpcResponse {
            jsonrpc: Some(JSONRPC_VERSION.to_string()),
            id,
            result: None,
            error: Some(Self::rpc_error(error)),
        }
    }

    fn rpc_error(error: Error) -> RpcError {
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
            Error::Protocol(message) => RpcError {
                code: ErrorCode::InvalidRequest.code(),
                message: message.to_string(),
                data: None,
            },
            Error::Io(err) => RpcError {
                code: ErrorCode::InternalError.code(),
                message: err.to_string(),
                data: None,
            },
        }
    }

    async fn write_response(&mut self, response: RpcResponse) -> Result<(), Error> {
        let stream = self
            .stream
            .as_mut()
            .ok_or(Error::Protocol("missing stream"))?;
        self.codec.write(stream, &response).await
    }
}

#[cfg(test)]
mod test;
