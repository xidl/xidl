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
    assert_eq!(decoded.try_as_file().unwrap().path, "x.rs");
}

#[test]
fn mismatched_artifact_and_input_variants_return_rpc_error() {
    let file = Artifact::new_file(ArtifactFile {
        path: "a.rs".into(),
        content: String::new(),
    });
    assert!(file.try_as_hir().is_err());
    assert!(file.try_as_rest_hir().is_err());
    assert!(file.try_as_jsonrpc_hir().is_err());
    assert!(file.clone().try_into_hir().is_err());
    assert!(file.clone().try_into_rest_hir().is_err());
    assert!(file.clone().try_into_jsonrpc_hir().is_err());
    assert_eq!(file.try_into_file().unwrap().path, "a.rs");

    let rpc_input = CodegenInput::new_rpc_hir(xidl_parser::hir::Specification(vec![]));
    assert!(rpc_input.clone().try_into_rest_hir().is_err());
    assert!(rpc_input.clone().try_into_jsonrpc_hir().is_err());
    assert!(rpc_input.clone().try_into_compilation().is_err());
    assert!(rpc_input.try_into_rpc_hir().is_ok());
}
