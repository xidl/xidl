use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{BufRead, Write};

const JSONRPC_VERSION: &str = "2.0";

#[derive(Debug, Clone, Copy)]
pub enum ErrorCode {
    ParseError,
    InvalidRequest,
    MethodNotFound,
    InvalidParams,
    InternalError,
    ServerError,
}

impl ErrorCode {
    pub fn code(self) -> i64 {
        match self {
            Self::ParseError => -32700,
            Self::InvalidRequest => -32600,
            Self::MethodNotFound => -32601,
            Self::InvalidParams => -32602,
            Self::InternalError => -32603,
            Self::ServerError => -32000,
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Json(serde_json::Error),
    Rpc {
        code: ErrorCode,
        message: String,
        data: Option<Value>,
    },
    Protocol(&'static str),
}

impl Error {
    pub fn method_not_found(method: &str) -> Self {
        Self::Rpc {
            code: ErrorCode::MethodNotFound,
            message: format!("method not found: {method}"),
            data: None,
        }
    }

    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::Rpc {
            code: ErrorCode::InvalidParams,
            message: message.into(),
            data: None,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "io error: {err}"),
            Self::Json(err) => write!(f, "json error: {err}"),
            Self::Rpc { code, message, .. } => {
                write!(f, "rpc error {}: {message}", code.code())
            }
            Self::Protocol(message) => write!(f, "protocol error: {message}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

#[derive(Serialize)]
struct RpcRequest<'a, P> {
    jsonrpc: &'static str,
    id: u64,
    method: &'a str,
    params: P,
}

#[derive(Deserialize)]
struct RpcRequestOwned {
    id: Option<u64>,
    method: Option<String>,
    params: Option<Value>,
}

#[derive(Serialize, Deserialize)]
struct RpcResponse {
    jsonrpc: Option<String>,
    id: Option<u64>,
    result: Option<Value>,
    error: Option<RpcError>,
}

#[derive(Serialize, Deserialize)]
struct RpcError {
    code: i64,
    message: String,
    data: Option<Value>,
}

pub struct Client<R, W> {
    reader: R,
    writer: W,
    next_id: u64,
}

impl<R, W> Client<R, W>
where
    R: BufRead,
    W: Write,
{
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader,
            writer,
            next_id: 1,
        }
    }

    pub fn call<P, T>(&mut self, method: &str, params: P) -> Result<T, Error>
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
        self.writer.write_all(payload.as_bytes())?;
        self.writer.write_all(b"\n")?;

        let mut line = String::new();
        let bytes = self.reader.read_line(&mut line)?;
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
        let result = response.result.ok_or(Error::Protocol("missing result"))?;
        Ok(serde_json::from_value(result)?)
    }
}

pub trait Handler {
    fn handle(&self, method: &str, params: Value) -> Result<Value, Error>;
}

pub fn serve<R, W, H>(mut reader: R, mut writer: W, handler: H) -> Result<(), Error>
where
    R: BufRead,
    W: Write,
    H: Handler,
{
    let mut line = String::new();
    loop {
        line.clear();
        let bytes = reader.read_line(&mut line)?;
        if bytes == 0 {
            break;
        }

        let request: RpcRequestOwned = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(err) => {
                write_error(&mut writer, None, Error::Json(err))?;
                continue;
            }
        };
        let id = request.id;
        let method = match request.method {
            Some(method) => method,
            None => {
                write_error(&mut writer, id, Error::Protocol("missing method"))?;
                continue;
            }
        };
        let params = request.params.unwrap_or(Value::Null);

        let result = handler.handle(&method, params);
        match result {
            Ok(value) => write_result(&mut writer, id, value)?,
            Err(err) => write_error(&mut writer, id, err)?,
        }
    }

    Ok(())
}

fn write_result<W: Write>(writer: &mut W, id: Option<u64>, result: Value) -> Result<(), Error> {
    let response = RpcResponse {
        jsonrpc: Some(JSONRPC_VERSION.to_string()),
        id,
        result: Some(result),
        error: None,
    };
    write_response(writer, response)
}

fn write_error<W: Write>(writer: &mut W, id: Option<u64>, error: Error) -> Result<(), Error> {
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
    write_response(writer, response)
}

fn write_response<W: Write>(writer: &mut W, response: RpcResponse) -> Result<(), Error> {
    let payload = serde_json::to_string(&response)?;
    writer.write_all(payload.as_bytes())?;
    writer.write_all(b"\n")?;
    Ok(())
}
