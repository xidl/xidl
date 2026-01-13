use crate::error::{IdlcError, Result};
use crate::generate::GeneratedFile;
use idl_rs::hir;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

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
}

#[derive(Deserialize)]
struct RpcResponse {
    jsonrpc: Option<String>,
    id: Option<u64>,
    result: Option<RpcResult>,
    error: Option<RpcError>,
}

#[derive(Deserialize)]
struct RpcResult {
    files: Vec<GeneratedFile>,
}

#[derive(Deserialize)]
struct RpcError {
    code: i64,
    message: String,
    data: Option<serde_json::Value>,
}

pub fn generate(lang: &str, spec: &hir::Specification) -> Result<Vec<GeneratedFile>> {
    let exe = format!("idlc-{lang}");
    let mut child = Command::new(&exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;

    let request = RpcRequest {
        jsonrpc: "2.0",
        id: 1,
        method: "generate",
        params: RpcParams { hir: spec },
    };
    let payload = serde_json::to_string(&request)?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(payload.as_bytes())?;
        stdin.write_all(b"\n")?;
        stdin.flush()?;
    }

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| IdlcError::rpc("missing plugin stdout"))?;
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    let bytes = reader.read_line(&mut line)?;
    if bytes == 0 {
        return Err(IdlcError::rpc("plugin returned no response"));
    }

    let response: RpcResponse = serde_json::from_str(&line)?;
    let status = child.wait()?;
    if !status.success() {
        return Err(IdlcError::rpc(format!("plugin exited with {status}")));
    }

    if let Some(error) = response.error {
        return Err(IdlcError::rpc(format!(
            "plugin error {}: {}",
            error.code, error.message
        )));
    }

    if response.id != Some(1) {
        return Err(IdlcError::rpc("unexpected JSON-RPC id"));
    }

    let result = response
        .result
        .ok_or_else(|| IdlcError::rpc("missing JSON-RPC result"))?;
    Ok(result.files)
}
