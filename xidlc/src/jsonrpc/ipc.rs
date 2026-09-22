//! Types and a small synchronous RPC contract for xidlc codegen plugins.
//!
//! This replaces the former generated JSON-RPC scaffolding. The wire protocol
//! is newline-delimited JSON, strictly request/response (one in flight):
//!
//! ```text
//! → {"id":1,"method":"get_engine_version"}
//! ← {"id":1,"result":"*"}
//! → {"id":2,"method":"get_properties"}
//! ← {"id":2,"result":{...}}
//! → {"id":3,"method":"generate","params":{"input":{...},"path":"a.idl","props":{...}}}
//! ← {"id":3,"result":[...]}
//! ← {"id":3,"error":{"message":"..."}}
//! ```
//!
//! External plugins speak this protocol on stdio (`xidl-<lang>` child process).
//! Built-in generators implement [`Codegen`] and are called directly in-process.

use serde::{Deserialize, Serialize};
use std::io::{BufRead, Write};
use xidl_parser::hir::ParserProperties;

/// Error carried across the plugin boundary.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RpcError {
    pub message: String,
}

impl RpcError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn method_not_found(method: &str) -> Self {
        Self::new(format!("method not found: {method}"))
    }

    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(format!("invalid params: {}", message.into()))
    }
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for RpcError {}

impl From<crate::error::IdlcError> for RpcError {
    fn from(err: crate::error::IdlcError) -> Self {
        Self::new(err.to_string())
    }
}

impl From<serde_json::Error> for RpcError {
    fn from(err: serde_json::Error) -> Self {
        Self::new(err.to_string())
    }
}

impl From<std::io::Error> for RpcError {
    fn from(err: std::io::Error) -> Self {
        Self::new(err.to_string())
    }
}

/// Generator stage contract.
///
/// Built-ins implement this and are dispatched in-process. External plugins
/// implement the NDJSON methods of the same names on stdio.
pub trait Codegen {
    fn get_engine_version(&self) -> Result<String, RpcError>;
    fn get_properties(&self) -> Result<ParserProperties, RpcError>;
    fn generate(
        &self,
        input: CodegenInput,
        path: String,
        props: ParserProperties,
    ) -> Result<Vec<Artifact>, RpcError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactHir {
    pub lang: String,
    pub hir: xidl_parser::hir::Specification,
    pub props: ParserProperties,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactRestHir {
    pub lang: String,
    pub rest_hir: xidl_parser::rest_hir::RestHirDocument,
    pub props: ParserProperties,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactJsonRpcHir {
    pub lang: String,
    pub jsonrpc_hir: xidl_parser::jsonrpc_hir::JsonRpcHirDocument,
    pub props: ParserProperties,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactFile {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactKind {
    Hir,
    RestHir,
    JsonRpcHir,
    File,
}

/// Intermediate pipeline payload. Stages either emit files or hand the next
/// semantic layer back to the driver.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum Artifact {
    Hir(ArtifactHir),
    RestHir(ArtifactRestHir),
    JsonRpcHir(ArtifactJsonRpcHir),
    File(ArtifactFile),
}

impl Artifact {
    pub fn new_hir(value: ArtifactHir) -> Self {
        Self::Hir(value)
    }

    pub fn new_rest_hir(value: ArtifactRestHir) -> Self {
        Self::RestHir(value)
    }

    pub fn new_jsonrpc_hir(value: ArtifactJsonRpcHir) -> Self {
        Self::JsonRpcHir(value)
    }

    pub fn new_file(value: ArtifactFile) -> Self {
        Self::File(value)
    }

    pub fn tag(&self) -> ArtifactKind {
        match self {
            Self::Hir(_) => ArtifactKind::Hir,
            Self::RestHir(_) => ArtifactKind::RestHir,
            Self::JsonRpcHir(_) => ArtifactKind::JsonRpcHir,
            Self::File(_) => ArtifactKind::File,
        }
    }

    pub fn is_hir(&self) -> bool {
        matches!(self, Self::Hir(_))
    }

    pub fn is_rest_hir(&self) -> bool {
        matches!(self, Self::RestHir(_))
    }

    pub fn is_jsonrpc_hir(&self) -> bool {
        matches!(self, Self::JsonRpcHir(_))
    }

    pub fn is_file(&self) -> bool {
        matches!(self, Self::File(_))
    }

    pub fn as_hir(&self) -> &ArtifactHir {
        match self {
            Self::Hir(value) => value,
            other => panic!("artifact is not Hir: {:?}", other.tag()),
        }
    }

    pub fn as_rest_hir(&self) -> &ArtifactRestHir {
        match self {
            Self::RestHir(value) => value,
            other => panic!("artifact is not RestHir: {:?}", other.tag()),
        }
    }

    pub fn as_jsonrpc_hir(&self) -> &ArtifactJsonRpcHir {
        match self {
            Self::JsonRpcHir(value) => value,
            other => panic!("artifact is not JsonRpcHir: {:?}", other.tag()),
        }
    }

    pub fn as_file(&self) -> &ArtifactFile {
        match self {
            Self::File(value) => value,
            other => panic!("artifact is not File: {:?}", other.tag()),
        }
    }

    pub fn into_hir(self) -> ArtifactHir {
        match self {
            Self::Hir(value) => value,
            other => panic!("artifact is not Hir: {:?}", other.tag()),
        }
    }

    pub fn into_rest_hir(self) -> ArtifactRestHir {
        match self {
            Self::RestHir(value) => value,
            other => panic!("artifact is not RestHir: {:?}", other.tag()),
        }
    }

    pub fn into_jsonrpc_hir(self) -> ArtifactJsonRpcHir {
        match self {
            Self::JsonRpcHir(value) => value,
            other => panic!("artifact is not JsonRpcHir: {:?}", other.tag()),
        }
    }

    pub fn into_file(self) -> ArtifactFile {
        match self {
            Self::File(value) => value,
            other => panic!("artifact is not File: {:?}", other.tag()),
        }
    }
}

/// Semantic input handed to a generator stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum CodegenInput {
    RpcHir(xidl_parser::hir::Specification),
    RestHir(xidl_parser::rest_hir::RestHirDocument),
    JsonRpcHir(xidl_parser::jsonrpc_hir::JsonRpcHirDocument),
}

impl CodegenInput {
    pub fn new_rpc_hir(value: xidl_parser::hir::Specification) -> Self {
        Self::RpcHir(value)
    }

    pub fn new_rest_hir(value: xidl_parser::rest_hir::RestHirDocument) -> Self {
        Self::RestHir(value)
    }

    pub fn new_jsonrpc_hir(value: xidl_parser::jsonrpc_hir::JsonRpcHirDocument) -> Self {
        Self::JsonRpcHir(value)
    }

    pub fn is_rpc_hir(&self) -> bool {
        matches!(self, Self::RpcHir(_))
    }

    pub fn is_rest_hir(&self) -> bool {
        matches!(self, Self::RestHir(_))
    }

    pub fn is_jsonrpc_hir(&self) -> bool {
        matches!(self, Self::JsonRpcHir(_))
    }

    pub fn into_rpc_hir(self) -> xidl_parser::hir::Specification {
        match self {
            Self::RpcHir(value) => value,
            _ => panic!("codegen input is not RpcHir"),
        }
    }

    pub fn into_rest_hir(self) -> xidl_parser::rest_hir::RestHirDocument {
        match self {
            Self::RestHir(value) => value,
            _ => panic!("codegen input is not RestHir"),
        }
    }

    pub fn into_jsonrpc_hir(self) -> xidl_parser::jsonrpc_hir::JsonRpcHirDocument {
        match self {
            Self::JsonRpcHir(value) => value,
            _ => panic!("codegen input is not JsonRpcHir"),
        }
    }
}

/// `generate` request payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateParams {
    pub input: CodegenInput,
    pub path: String,
    pub props: ParserProperties,
}

/// Wire request. `params` is a JSON object (or null for no-arg methods).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest {
    pub id: u64,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

/// Wire response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    pub id: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

fn dispatch(handler: &dyn Codegen, request: RpcRequest) -> RpcResponse {
    let outcome = match request.method.as_str() {
        "get_engine_version" => handler
            .get_engine_version()
            .and_then(|value| Ok(serde_json::to_value(value)?)),
        "get_properties" => handler
            .get_properties()
            .and_then(|value| Ok(serde_json::to_value(value)?)),
        "generate" => serde_json::from_value::<GenerateParams>(request.params)
            .map_err(RpcError::from)
            .and_then(|params| {
                handler
                    .generate(params.input, params.path, params.props)
                    .and_then(|value| Ok(serde_json::to_value(value)?))
            }),
        other => Err(RpcError::method_not_found(other)),
    };
    match outcome {
        Ok(result) => RpcResponse {
            id: request.id,
            result: Some(result),
            error: None,
        },
        Err(error) => RpcResponse {
            id: request.id,
            result: None,
            error: Some(error),
        },
    }
}

/// Serve [`Codegen`] on stdin/stdout using the NDJSON protocol.
///
/// Used by external plugin binaries (`xidl-<lang>`).
pub fn serve_stdio(handler: impl Codegen + Send + 'static) -> std::io::Result<()> {
    serve_stdio_dyn(Box::new(handler))
}

/// Object-safe variant of [`serve_stdio`].
pub fn serve_stdio_dyn(handler: Box<dyn Codegen + Send>) -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<RpcRequest>(trimmed) {
            Ok(request) => dispatch(handler.as_ref(), request),
            Err(err) => RpcResponse {
                id: 0,
                result: None,
                error: Some(RpcError::from(err)),
            },
        };
        writeln!(writer, "{}", serde_json::to_string(&response)?)?;
        writer.flush()?;
    }
    Ok(())
}

/// Client half of the NDJSON protocol, talking to an external plugin child
/// process over its stdio pipes.
pub struct PluginClient {
    child: std::process::Child,
    stdin: Option<std::process::ChildStdin>,
    stdout: std::io::BufReader<std::process::ChildStdout>,
    next_id: u64,
}

impl PluginClient {
    pub fn spawn(exe: &str) -> std::io::Result<Self> {
        use std::process::Stdio;
        let mut child = std::process::Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        let stdin = child.stdin.take();
        let stdout = child.stdout.take();
        Ok(Self {
            child,
            stdin,
            stdout: std::io::BufReader::new(stdout.expect("piped stdout")),
            next_id: 1,
        })
    }

    pub fn call<T>(&mut self, method: &str, params: serde_json::Value) -> Result<T, RpcError>
    where
        T: serde::de::DeserializeOwned,
    {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        let request = serde_json::json!({
            "id": id,
            "method": method,
            "params": params,
        });
        {
            let stdin = self
                .stdin
                .as_mut()
                .ok_or_else(|| RpcError::new("plugin stdin already closed"))?;
            writeln!(stdin, "{}", serde_json::to_string(&request)?)?;
            stdin.flush()?;
        }

        let mut line = String::new();
        let read = self.stdout.read_line(&mut line)?;
        if read == 0 {
            return Err(RpcError::new("plugin closed the response stream"));
        }
        let response: RpcResponse = serde_json::from_str(line.trim())?;
        if response.id != id {
            return Err(RpcError::new(format!(
                "plugin response id mismatch: expected {id}, got {}",
                response.id
            )));
        }
        if let Some(error) = response.error {
            return Err(error);
        }
        let value = response.result.unwrap_or(serde_json::Value::Null);
        Ok(serde_json::from_value(value)?)
    }

    /// Close stdin and wait for the plugin to exit.
    pub fn shutdown(&mut self) {
        self.stdin.take();
        let _ = self.child.wait();
    }
}

impl Drop for PluginClient {
    fn drop(&mut self) {
        self.stdin.take();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Echo;

    impl Codegen for Echo {
        fn get_engine_version(&self) -> Result<String, RpcError> {
            Ok("^0.92".to_string())
        }

        fn get_properties(&self) -> Result<ParserProperties, RpcError> {
            let mut props = ParserProperties::new();
            props.insert("k".to_string(), serde_json::json!("v"));
            Ok(props)
        }

        fn generate(
            &self,
            input: CodegenInput,
            path: String,
            props: ParserProperties,
        ) -> Result<Vec<Artifact>, RpcError> {
            let _ = input;
            Ok(vec![Artifact::new_file(ArtifactFile {
                path,
                content: props
                    .get("k")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null)
                    .to_string(),
            })])
        }
    }

    fn handle_line(handler: &dyn Codegen, line: &str) -> String {
        let request: RpcRequest = serde_json::from_str(line).unwrap();
        let response = dispatch(handler, request);
        serde_json::to_string(&response).unwrap()
    }

    #[test]
    fn dispatches_unary_methods() {
        let handler = Echo;
        let version = handle_line(&handler, r#"{"id":1,"method":"get_engine_version"}"#);
        assert!(version.contains(r#""result":"^0.92""#), "{version}");

        let props = handle_line(&handler, r#"{"id":2,"method":"get_properties"}"#);
        assert!(props.contains(r#""k":"v""#), "{props}");
    }

    #[test]
    fn dispatches_generate_with_tagged_payload() {
        let handler = Echo;
        let request = serde_json::json!({
            "id": 7,
            "method": "generate",
            "params": {
                "input": {"kind": "RpcHir", "value": []},
                "path": "a.idl",
                "props": {"k": "v"},
            }
        });
        let response = handle_line(&handler, &request.to_string());
        assert!(response.contains(r#""id":7"#), "{response}");
        assert!(response.contains("a.idl"), "{response}");
    }

    #[test]
    fn reports_unknown_method() {
        let handler = Echo;
        let response = handle_line(&handler, r#"{"id":3,"method":"nope"}"#);
        assert!(response.contains("method not found"), "{response}");
    }

    #[test]
    fn artifact_round_trips_through_json() {
        let artifact = Artifact::new_file(ArtifactFile {
            path: "x.rs".to_string(),
            content: "fn main() {}".to_string(),
        });
        let encoded = serde_json::to_string(&artifact).unwrap();
        assert!(encoded.contains(r#""kind":"File""#), "{encoded}");
        let decoded: Artifact = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.as_file().path, "x.rs");
    }
}
