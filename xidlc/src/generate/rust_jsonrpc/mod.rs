mod definition;
mod interface;
mod render;
mod spec;

use crate::error::IdlcResult;
use crate::generate::GeneratedFile;
use serde_json::json;
use std::path::Path;
use xidl_parser::hir;
use xidl_parser::hir::{ParserProperties, Specification};

pub use render::{JsonRpcRender, JsonRpcRenderOutput, JsonRpcRenderer};

pub fn generate(spec: &hir::Specification, input_path: &Path) -> IdlcResult<Vec<GeneratedFile>> {
    let stem = input_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("output");
    let _base = crate::generate::to_snake_case(stem);

    let file_name = input_path.file_stem().unwrap().to_str().unwrap();
    let filename = format!("{file_name}.rs");

    let renderer = JsonRpcRenderer::new()?;
    let output = spec.render(&renderer)?;

    let source = renderer.render_template(
        "spec.rs.j2",
        &json!({
            "definitions": output.source,
        }),
    )?;

    Ok(vec![GeneratedFile {
        filename,
        filecontent: source,
    }])
}

pub fn serve_jsonrpc<R: std::io::BufRead, W: std::io::Write>(
    reader: R,
    writer: W,
) -> IdlcResult<()> {
    let handler = crate::jsonrpc::CodegenServer::new(RustJsonRpcCodegen);
    xidl_jsonrpc::serve(reader, writer, handler)
        .map_err(|err| crate::error::IdlcError::rpc(err.to_string()))
}

struct RustJsonRpcCodegen;

impl crate::jsonrpc::Codegen for RustJsonRpcCodegen {
    fn get_properties(&self) -> Result<ParserProperties, xidl_jsonrpc::Error> {
        Ok(ParserProperties::default())
    }

    fn generate(
        &self,
        hir: Specification,
        input: String,
    ) -> Result<Vec<GeneratedFile>, xidl_jsonrpc::Error> {
        generate(&hir, Path::new(&input)).map_err(map_codegen_error)
    }
}

fn map_codegen_error(err: crate::error::IdlcError) -> xidl_jsonrpc::Error {
    xidl_jsonrpc::Error::Rpc {
        code: xidl_jsonrpc::ErrorCode::ServerError,
        message: err.to_string(),
        data: None,
    }
}
