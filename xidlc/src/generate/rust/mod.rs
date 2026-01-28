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
pub(crate) mod util;

pub use render::{RustRender, RustRenderOutput, RustRenderer};

use crate::error::IdlcResult;
use crate::generate::Artifact;
use serde_json::json;
use std::path::Path;
use xidl_parser::hir;
use xidl_parser::hir::{ParserProperties, Specification};

pub fn generate_with_properties(
    spec: &hir::Specification,
    input_path: &Path,
    properties: &ParserProperties,
) -> IdlcResult<Vec<Artifact>> {
    let stem = input_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("output");
    let base = crate::generate::to_snake_case(stem);
    let filename = format!("{base}.rs");

    let typeobject_path = "xidl_typeobject";

    let renderer = RustRenderer::new(typeobject_path.to_string(), properties.clone())?;
    let output = spec.render(&renderer)?;

    let source = renderer.render_template(
        "spec.rs.j2",
        &json!({
            "definitions": output.source,
        }),
    )?;

    Ok(vec![Artifact::File {
        path: filename,
        content: source,
    }])
}

pub fn serve_jsonrpc<R: std::io::BufRead, W: std::io::Write>(
    reader: R,
    writer: W,
) -> IdlcResult<()> {
    let handler = crate::jsonrpc::CodegenServer::new(RustCodegen {
        render_header: true,
    });
    xidl_jsonrpc::serve(reader, writer, handler)
        .map_err(|err| crate::error::IdlcError::rpc(err.to_string()))
}

struct RustCodegen {
    render_header: bool,
}

impl crate::jsonrpc::Codegen for RustCodegen {
    fn get_properties(&self) -> Result<ParserProperties, xidl_jsonrpc::Error> {
        Ok(ParserProperties::default())
    }

    fn generate(
        &self,
        hir: Specification,
        input: String,
    ) -> Result<Vec<Artifact>, xidl_jsonrpc::Error> {
        let mut properties = ParserProperties::default();
        properties.insert(
            "render_header".to_string(),
            serde_json::Value::Bool(self.render_header),
        );
        generate_with_properties(&hir, Path::new(&input), &properties).map_err(map_codegen_error)
    }
}

fn map_codegen_error(err: crate::error::IdlcError) -> xidl_jsonrpc::Error {
    xidl_jsonrpc::Error::Rpc {
        code: xidl_jsonrpc::ErrorCode::ServerError,
        message: err.to_string(),
        data: None,
    }
}
