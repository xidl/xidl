use crate::line_io::write_json_line;
use crate::{Error, ErrorCode, Handler, JSONRPC_VERSION, RpcError, RpcRequestOwned, RpcResponse};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, BufStream};

pub(crate) struct ServerSession<RW, H> {
    stream: Option<BufStream<RW>>,
    handler: H,
}

impl<RW, H> ServerSession<RW, H>
where
    H: Handler,
    RW: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    pub(crate) fn new(stream: RW, handler: H) -> Self {
        let stream = tokio::io::BufStream::new(stream);
        Self {
            stream: Some(stream),
            handler,
        }
    }

    pub(crate) async fn run(&mut self) -> Result<(), Error> {
        let mut line = String::new();
        loop {
            line.clear();
            let Some(stream) = self.stream.as_mut() else {
                break;
            };
            let bytes = stream.read_line(&mut line).await?;
            if bytes == 0 {
                break;
            }
            if !self.handle_line(&line).await? {
                break;
            }
        }
        Ok(())
    }

    async fn handle_line(&mut self, line: &str) -> Result<bool, Error> {
        let request: RpcRequestOwned = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(err) => {
                self.write_error(None, Error::Json(err)).await?;
                return Ok(true);
            }
        };
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
            let bidi = crate::stream::open_bidi_server(stream);
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
        write_json_line(stream, &response).await
    }
}
