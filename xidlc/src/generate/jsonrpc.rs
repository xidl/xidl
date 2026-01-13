use crate::error::{IdlcError, IdlcResult};
use crate::generate::GeneratedFile;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, Write};
use xidl_parser::hir;

const JSONRPC_VERSION: &str = "2.0";

#[derive(Serialize)]
struct RpcRequest<'a> {
    jsonrpc: &'static str,
    id: u64,
    method: &'static str,
    params: RpcParams<'a>,
}

#[derive(Serialize)]
struct RpcParams<'a> {
    hir: &'a hir::Specification,
    input: Option<&'a str>,
}

#[derive(Deserialize)]
struct RpcRequestOwned {
    id: Option<u64>,
    method: Option<String>,
    params: RpcParamsOwned,
}

#[derive(Deserialize)]
struct RpcParamsOwned {
    hir: hir::Specification,
    input: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct RpcResponse {
    jsonrpc: Option<String>,
    id: Option<u64>,
    result: Option<RpcResult>,
    error: Option<RpcError>,
}

#[derive(Serialize, Deserialize)]
struct RpcResult {
    files: Vec<GeneratedFile>,
}

#[derive(Serialize, Deserialize)]
struct RpcError {
    code: i64,
    message: String,
    data: Option<serde_json::Value>,
}

pub struct JsonRpcClient<R, W> {
    reader: R,
    writer: W,
    next_id: u64,
}

impl<R, W> JsonRpcClient<R, W>
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

    pub fn generate(
        &mut self,
        spec: &hir::Specification,
        input: Option<&str>,
    ) -> IdlcResult<Vec<GeneratedFile>> {
        let id = self.next_id;
        self.next_id += 1;

        let request = RpcRequest {
            jsonrpc: JSONRPC_VERSION,
            id,
            method: "generate",
            params: RpcParams { hir: spec, input },
        };
        let payload = serde_json::to_string(&request)?;
        self.writer.write_all(payload.as_bytes()).unwrap();
        self.writer.write_all(b"\n").unwrap();

        let mut line = String::new();
        let bytes = self.reader.read_line(&mut line)?;
        if bytes == 0 {
            return Err(IdlcError::rpc("plugin returned no response"));
        }

        let response: RpcResponse = serde_json::from_str(&line)?;
        if let Some(error) = response.error {
            return Err(IdlcError::rpc(format!(
                "plugin error {}: {}",
                error.code, error.message
            )));
        }

        if response.id != Some(id) {
            return Err(IdlcError::rpc("unexpected JSON-RPC id"));
        }

        let result = response
            .result
            .ok_or_else(|| IdlcError::rpc("missing JSON-RPC result"))?;
        Ok(result.files)
    }
}

pub fn serve_generate<R, W, F>(mut reader: R, mut writer: W, handler: F) -> IdlcResult<()>
where
    R: BufRead,
    W: Write,
    F: FnOnce(&hir::Specification, Option<&str>) -> IdlcResult<Vec<GeneratedFile>>,
{
    let mut line = String::new();
    let bytes = reader.read_line(&mut line)?;
    if bytes == 0 {
        return Err(IdlcError::rpc("client returned no request"));
    }

    let request: RpcRequestOwned = serde_json::from_str(&line)?;
    let id = request.id.unwrap_or(0);
    if request.method.as_deref() != Some("generate") {
        return write_error(&mut writer, id, "unknown method");
    }

    let result = handler(&request.params.hir, request.params.input.as_deref());
    match result {
        Ok(files) => write_result(&mut writer, id, files),
        Err(err) => write_error(&mut writer, id, &err.to_string()),
    }
}

fn write_result<W: Write>(writer: &mut W, id: u64, files: Vec<GeneratedFile>) -> IdlcResult<()> {
    let response = RpcResponse {
        jsonrpc: Some(JSONRPC_VERSION.to_string()),
        id: Some(id),
        result: Some(RpcResult { files }),
        error: None,
    };
    write_response(writer, response)
}

fn write_error<W: Write>(writer: &mut W, id: u64, message: &str) -> IdlcResult<()> {
    let response = RpcResponse {
        jsonrpc: Some(JSONRPC_VERSION.to_string()),
        id: Some(id),
        result: None,
        error: Some(RpcError {
            code: -32000,
            message: message.to_string(),
            data: None,
        }),
    };
    write_response(writer, response)
}

fn write_response<W: Write>(writer: &mut W, response: RpcResponse) -> IdlcResult<()> {
    let payload = serde_json::to_string(&response)?;
    writer.write_all(payload.as_bytes())?;
    writer.write_all(b"\n")?;
    Ok(())
}
