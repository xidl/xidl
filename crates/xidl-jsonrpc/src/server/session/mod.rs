use crate::codec::Codec;
use crate::{Error, ErrorCode, Handler, JSONRPC_VERSION, RpcError, RpcResponse};
use serde_json::Value;
use tokio::io::{AsyncRead, AsyncWrite, BufStream};

/// A parsed JSON-RPC request with the id separated from the payload.
struct ParsedRequest {
    id: Option<Value>,
    method: String,
    params: Value,
}

/// A request that failed structural validation.
struct InvalidRequest {
    id: Option<Value>,
    reason: &'static str,
}

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
            let value = match self.codec.read::<_, Value>(stream).await {
                Ok(Some(value)) => value,
                Ok(None) => break,
                Err(err) if Self::is_decode_error(&err) => {
                    self.write_error(None, err).await?;
                    continue;
                }
                Err(err) => return Err(err),
            };
            let continue_loop = match value {
                Value::Array(items) => self.handle_batch(items).await?,
                other => {
                    self.handle_value(other).await?;
                    true
                }
            };
            if !continue_loop {
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
            Error::FrameTooLarge { .. } => true,
            _ => false,
        }
    }

    fn parse_request(value: Value) -> Result<ParsedRequest, InvalidRequest> {
        let Value::Object(mut map) = value else {
            return Err(InvalidRequest {
                id: None,
                reason: "request must be an object",
            });
        };
        let version_ok = map
            .get("jsonrpc")
            .is_some_and(|value| value.as_str() == Some(JSONRPC_VERSION));
        if !version_ok {
            return Err(InvalidRequest {
                id: map.get("id").cloned(),
                reason: "missing or invalid jsonrpc version",
            });
        }
        let Some(method) = map
            .remove("method")
            .and_then(|value| value.as_str().map(str::to_string))
        else {
            return Err(InvalidRequest {
                id: map.get("id").cloned(),
                reason: "missing method",
            });
        };
        let id = map.remove("id");
        let params = map.remove("params").unwrap_or(Value::Null);
        Ok(ParsedRequest { id, method, params })
    }

    async fn handle_value(&mut self, value: Value) -> Result<(), Error> {
        match Self::parse_request(value) {
            Ok(request) => {
                self.handle_request(request).await?;
            }
            Err(invalid) => {
                self.write_error(invalid.id, Error::Protocol(invalid.reason))
                    .await?;
            }
        }
        Ok(())
    }

    async fn handle_batch(&mut self, items: Vec<Value>) -> Result<bool, Error> {
        if items.is_empty() {
            self.write_error(None, Error::Protocol("empty batch"))
                .await?;
            return Ok(true);
        }

        let mut responses = Vec::new();
        for item in items {
            let request = match Self::parse_request(item) {
                Ok(request) => request,
                Err(invalid) => {
                    responses.push(Self::error_response(
                        invalid.id,
                        Error::Protocol(invalid.reason),
                    ));
                    continue;
                }
            };
            if self.handler.accepts_bidi(&request.method) {
                if request.id.is_some() {
                    responses.push(Self::error_response(
                        request.id,
                        Error::Protocol("bidi method not allowed in batch"),
                    ));
                }
                continue;
            }
            let response = match self.handler.handle(&request.method, request.params).await {
                Ok(value) => Self::success_response(request.id, value),
                Err(err) => Self::error_response(request.id, err),
            };
            if response.id.is_some() {
                responses.push(response);
            }
        }

        if !responses.is_empty() {
            self.write_responses(responses).await?;
        }
        Ok(true)
    }

    async fn handle_request(&mut self, request: ParsedRequest) -> Result<bool, Error> {
        if self.handler.accepts_bidi(&request.method) {
            if request.id.is_some() {
                self.write_result(request.id, Value::Null).await?;
            }
            let stream = self
                .stream
                .take()
                .ok_or(Error::Protocol("missing stream"))?;
            let bidi = crate::stream::open_bidi_server_with(stream, self.codec);
            self.handler
                .handle_bidi(&request.method, request.params, bidi)
                .await?;
            return Ok(false);
        }

        match self.handler.handle(&request.method, request.params).await {
            Ok(value) => {
                if let Some(id) = request.id {
                    self.write_result(Some(id), value).await?;
                }
            }
            Err(err) => {
                if let Some(id) = request.id {
                    self.write_error(Some(id), err).await?;
                }
            }
        }
        Ok(true)
    }

    async fn write_result(&mut self, id: Option<Value>, result: Value) -> Result<(), Error> {
        self.write_response(Self::success_response(id, result))
            .await
    }

    async fn write_error(&mut self, id: Option<Value>, error: Error) -> Result<(), Error> {
        self.write_response(Self::error_response(id, error)).await
    }

    fn success_response(id: Option<Value>, result: Value) -> RpcResponse {
        RpcResponse {
            jsonrpc: Some(JSONRPC_VERSION.to_string()),
            id,
            result: Some(result),
            error: None,
        }
    }

    fn error_response(id: Option<Value>, error: Error) -> RpcResponse {
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

    async fn write_responses(&mut self, responses: Vec<RpcResponse>) -> Result<(), Error> {
        let stream = self
            .stream
            .as_mut()
            .ok_or(Error::Protocol("missing stream"))?;
        self.codec.write(stream, &responses).await
    }
}

#[cfg(test)]
mod test;
