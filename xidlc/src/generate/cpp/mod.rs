mod bitmask_dcl;
mod bitset_dcl;
mod const_dcl;
mod constr_type;
mod definition;
mod enum_dcl;
mod exception_dcl;
mod interface_dcl;
mod render;
mod spec;
mod struct_dcl;
mod type_dcl;
mod union_def;
mod util;

pub use render::{CppRender, CppRenderOutput, CppRenderer};

use crate::error::IdlcResult;
use crate::generate::GeneratedFile;
use serde_json::json;
use std::path::Path;
use xidl_parser::hir;
use xidl_parser::hir::{ParserProperties, Specification};

pub fn generate(spec: &hir::Specification, input_path: &Path) -> IdlcResult<Vec<GeneratedFile>> {
    let stem = input_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("output");
    let base = crate::generate::to_snake_case(stem);
    let header_name = format!("{base}.h");

    let renderer = CppRenderer::new()?;
    let output = spec.render(&renderer)?;

    let header = renderer.render_template(
        "spec.h.j2",
        &json!({
            "definitions": output.header,
            "filename": header_name,
        }),
    )?;
    let source = renderer.render_template(
        "spec.cpp.j2",
        &json!({
            "header_name": header_name,
            "definitions": output.source,
        }),
    )?;

    Ok(vec![
        GeneratedFile {
            filename: header_name.clone(),
            filecontent: header,
        },
        GeneratedFile {
            filename: format!("{base}.cpp"),
            filecontent: source,
        },
    ])
}

pub fn serve_jsonrpc<R: std::io::BufRead, W: std::io::Write>(
    reader: R,
    writer: W,
) -> IdlcResult<()> {
    let handler = crate::jsonrpc::CodegenServer::new(CppCodegen);
    xidl_jsonrpc::serve(reader, writer, handler)
        .map_err(|err| crate::error::IdlcError::rpc(err.to_string()))
}

struct CppCodegen;

impl crate::jsonrpc::Codegen for CppCodegen {
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
        code: -32000,
        message: err.to_string(),
        data: None,
    }
}
