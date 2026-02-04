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

use convert_case::{Case, Casing};
pub use render::{RustRender, RustRenderOutput, RustRenderer};

use crate::error::IdlcResult;
use crate::jsonrpc::{Artifact, ArtifactFile};
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
    let base = stem.to_case(Case::Snake);
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
    let source = maybe_format_rust(source, properties)?;

    Ok(vec![Artifact::new_file(ArtifactFile {
        path: filename,
        content: source,
    })])
}

pub(crate) struct RustCodegen;

impl crate::jsonrpc::Codegen for RustCodegen {
    fn get_engine_version(&self) -> Result<String, xidl_jsonrpc::Error> {
        Ok(crate::generate::compatible_xidlc_version())
    }

    fn get_properties(&self) -> Result<ParserProperties, xidl_jsonrpc::Error> {
        Ok(crate::macros::hashmap! {
            "format_rust" => true,
            "enable_render_header" => true,
            "enable_serialize" => true,
            "enable_deserialize" => true,
            "enable_metadata" => true
        })
    }

    fn generate(
        &self,
        hir: Specification,
        input: String,
        props: ::xidl_parser::hir::ParserProperties,
    ) -> Result<Vec<Artifact>, xidl_jsonrpc::Error> {
        generate_with_properties(&hir, Path::new(&input), &props).map_err(map_codegen_error)
    }
}

fn map_codegen_error(err: crate::error::IdlcError) -> xidl_jsonrpc::Error {
    xidl_jsonrpc::Error::Rpc {
        code: xidl_jsonrpc::ErrorCode::ServerError,
        message: err.to_string(),
        data: None,
    }
}

fn maybe_format_rust(source: String, properties: &ParserProperties) -> IdlcResult<String> {
    let format = properties
        .get("format_rust")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    if format {
        crate::fmt::format_rust_source(&source)
    } else {
        Ok(source)
    }
}
