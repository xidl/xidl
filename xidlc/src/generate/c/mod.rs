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
mod xcdr;

pub use render::{CRender, CRenderOutput, CRenderer};

use crate::error::IdlcResult;
use crate::generate::Artifact;
use serde_json::json;
use std::path::Path;
use xidl_parser::hir;
use xidl_parser::hir::{ParserProperties, Specification};

pub fn generate(spec: &hir::Specification, input_path: &Path) -> IdlcResult<Vec<Artifact>> {
    let stem = input_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("output");
    let base = crate::generate::to_snake_case(stem);
    let filename = format!("{base}.h");
    let xcdr_header_name = format!("{base}_xcdr.h");

    let renderer = CRenderer::new()?;
    let output = spec.render(&renderer)?;

    let header = renderer.render_template(
        "spec.h.j2",
        &json!({
            "definitions": output.header,
            "filename": filename,
        }),
    )?;
    let source = renderer.render_template(
        "spec.c.j2",
        &json!({
            "header_name": filename,
            "definitions": output.source,
        }),
    )?;
    let xcdr_header = renderer.render_template(
        "spec.h.j2",
        &json!({
            "definitions": output.xcdr_header,
            "filename": xcdr_header_name,
        }),
    )?;
    let xcdr_source = renderer.render_template(
        "spec.c.j2",
        &json!({
            "header_name": xcdr_header_name,
            "definitions": output.xcdr_source,
        }),
    )?;

    Ok(vec![
        Artifact::File {
            path: filename.clone(),
            content: header,
        },
        Artifact::File {
            path: format!("{base}.c"),
            content: source,
        },
        Artifact::File {
            path: xcdr_header_name.clone(),
            content: xcdr_header,
        },
        Artifact::File {
            path: format!("{base}_xcdr.c"),
            content: xcdr_source,
        },
    ])
}

pub fn serve_jsonrpc<R: std::io::BufRead, W: std::io::Write>(
    reader: R,
    writer: W,
) -> IdlcResult<()> {
    let handler = crate::jsonrpc::CodegenServer::new(CCodegen);
    xidl_jsonrpc::serve(reader, writer, handler)
        .map_err(|err| crate::error::IdlcError::rpc(err.to_string()))
}

struct CCodegen;

impl crate::jsonrpc::Codegen for CCodegen {
    fn get_properties(&self) -> Result<ParserProperties, xidl_jsonrpc::Error> {
        Ok(ParserProperties::default())
    }

    fn generate(
        &self,
        hir: Specification,
        input: String,
    ) -> Result<Vec<Artifact>, xidl_jsonrpc::Error> {
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
