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

use convert_case::{Case, Casing};
pub use render::{CRender, CRenderOutput, CRenderer};

use crate::error::IdlcResult;
use crate::jsonrpc::{Artifact, ArtifactFile};
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;
use xidl_parser::hir;
use xidl_parser::hir::{ParserProperties, Specification};

pub fn generate(
    spec: &hir::Specification,
    input_path: &Path,
    props: HashMap<String, serde_json::Value>,
) -> IdlcResult<Vec<Artifact>> {
    let stem = input_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("output");
    let base = stem.to_case(Case::Snake);
    let filename = format!("{base}.h");
    let xcdr_header_name = format!("{base}_xcdr.h");

    let mut renderer = CRenderer::new()?;
    renderer
        .env()
        .add_global("opt".to_string(), minijinja::Value::from_serialize(&props));
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
    let header = maybe_format_c(header, &props)?;
    let source = maybe_format_c(source, &props)?;
    let xcdr_header = maybe_format_c(xcdr_header, &props)?;
    let xcdr_source = maybe_format_c(xcdr_source, &props)?;

    Ok(vec![
        Artifact::new_file(ArtifactFile {
            path: filename.clone(),
            content: header,
        }),
        Artifact::new_file(ArtifactFile {
            path: format!("{base}.c"),
            content: source,
        }),
        Artifact::new_file(ArtifactFile {
            path: xcdr_header_name.clone(),
            content: xcdr_header,
        }),
        Artifact::new_file(ArtifactFile {
            path: format!("{base}_xcdr.c"),
            content: xcdr_source,
        }),
    ])
}

pub(crate) struct CCodegen;

impl crate::jsonrpc::Codegen for CCodegen {
    fn get_engine_version(&self) -> Result<String, xidl_jsonrpc::Error> {
        Ok(crate::generate::compatible_xidlc_version())
    }

    fn get_properties(&self) -> Result<ParserProperties, xidl_jsonrpc::Error> {
        Ok(crate::macros::hashmap! {
            "format_c" => true
        })
    }

    fn generate(
        &self,
        hir: Specification,
        path: String,
        props: ::xidl_parser::hir::ParserProperties,
    ) -> Result<Vec<Artifact>, xidl_jsonrpc::Error> {
        generate(&hir, Path::new(&path), props).map_err(map_codegen_error)
    }
}

fn map_codegen_error(err: crate::error::IdlcError) -> xidl_jsonrpc::Error {
    xidl_jsonrpc::Error::Rpc {
        code: xidl_jsonrpc::ErrorCode::ServerError,
        message: err.to_string(),
        data: None,
    }
}

fn maybe_format_c(
    source: String,
    properties: &HashMap<String, serde_json::Value>,
) -> IdlcResult<String> {
    let format = properties
        .get("format_c")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    if format {
        crate::fmt::format_c_source(&source)
    } else {
        Ok(source)
    }
}
