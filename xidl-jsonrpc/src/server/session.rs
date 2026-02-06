use crate::{Error, ErrorCode, Handler, RpcError, RpcRequestOwned, RpcResponse, JSONRPC_VERSION};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufStream};

pub(crate) struct ServerSession<RW, H> {
    stream: BufStream<RW>,
    handler: H,
}

impl<RW, H> ServerSession<RW, H>
where
    H: Handler,
    RW: AsyncRead + AsyncWrite + Unpin,
{
    pub(crate) fn new(stream: RW, handler: H) -> Self {
        let stream = tokio::io::BufStream::new(stream);
        Self { stream, handler }
    }

    pub(crate) async fn run(&mut self) -> Result<(), Error> {
        let mut line = String::new();
        loop {
            line.clear();
            let bytes = self.stream.read_line(&mut line).await?;
            if bytes == 0 {
                break;
            }
            self.handle_line(&line).await?;
        }
        Ok(())
    }

    async fn handle_line(&mut self, line: &str) -> Result<(), Error> {
        let request: RpcRequestOwned = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(err) => {
                self.write_error(None, Error::Json(err)).await?;
                return Ok(());
            }
        };
        let id = request.id;
        let method = match request.method {
            Some(method) => method,
            None => {
                self.write_error(id, Error::Protocol("missing method"))
                    .await?;
                return Ok(());
            }
        };
        let params = request.params.unwrap_or(Value::Null);

        match self.handler.handle(&method, params).await {
            Ok(value) => self.write_result(id, value).await,
            Err(err) => self.write_error(id, err).await,
        }
    }

    async fn write_result(&mut self, id: Option<u64>, result: Value) -> Result<(), Error> {
        let response = RpcResponse {
            jsonrpc: Some(JSONRPC_VERSION.to_string()),
            id,
            result: Some(result),
            error: None,
        };
        self.write_response(response).await
    }

    async fn write_error(&mut self, id: Option<u64>, error: Error) -> Result<(), Error> {
        let rpc_error = match error {
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
        };
        let response = RpcResponse {
            jsonrpc: Some(JSONRPC_VERSION.to_string()),
            id,
            result: None,
            error: Some(rpc_error),
        };
        self.write_response(response).await
    }

    async fn write_response(&mut self, response: RpcResponse) -> Result<(), Error> {
        let payload = serde_json::to_string(&response)?;
        self.stream.write_all(payload.as_bytes()).await?;
        self.stream.write_all(b"\n").await?;
        self.stream.flush().await?;
        Ok(())
    }
}
