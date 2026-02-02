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

use convert_case::{Case, Casing};
pub use render::{CppRender, CppRenderOutput, CppRenderer};

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
    let header_name = format!("{base}.h");

    let mut renderer = CppRenderer::new()?;
    for (k, v) in props {
        renderer
            .env()
            .add_global(k, minijinja::Value::from_serialize(v));
    }

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
        Artifact::new_file(ArtifactFile {
            path: header_name.clone(),
            content: header,
        }),
        Artifact::new_file(ArtifactFile {
            path: format!("{base}.cpp"),
            content: source,
        }),
    ])
}

pub(crate) struct CppCodegen;

impl crate::jsonrpc::Codegen for CppCodegen {
    fn get_properties(&self) -> Result<ParserProperties, xidl_jsonrpc::Error> {
        Ok(ParserProperties::default())
    }

    fn generate(
        &self,
        hir: Specification,
        input: String,
        props: ::xidl_parser::hir::ParserProperties,
    ) -> Result<Vec<Artifact>, xidl_jsonrpc::Error> {
        generate(&hir, Path::new(&input), props).map_err(map_codegen_error)
    }
}

fn map_codegen_error(err: crate::error::IdlcError) -> xidl_jsonrpc::Error {
    xidl_jsonrpc::Error::Rpc {
        code: xidl_jsonrpc::ErrorCode::ServerError,
        message: err.to_string(),
        data: None,
    }
}
